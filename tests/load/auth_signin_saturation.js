import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

const baseUrl = __ENV.AUTH_API_URL || 'http://127.0.0.1:8080';
const publishableKey = __ENV.PUBLISHABLE_KEY || 'pk_test_replace_me';
const testEmail = __ENV.LOAD_TEST_EMAIL || 'loadtest@example.com';
const testPassword = __ENV.LOAD_TEST_PASSWORD || 'LoadTestPassword!2026';

const signinDuration = new Trend('auth_signin_duration', true);
const jwksDuration = new Trend('jwks_read_duration', true);
const healthDuration = new Trend('health_probe_duration', true);
const controlPlaneAvailability = new Rate('fast_path_available');

export const options = {
  scenarios: {
    argon2_signin_saturation: {
      executor: 'ramping-arrival-rate',
      startRate: 5,
      timeUnit: '1s',
      preAllocatedVUs: 25,
      maxVUs: 100,
      stages: [
        { duration: '20s', target: 20 },
        { duration: '60s', target: 40 },
        { duration: '20s', target: 5 },
      ],
      exec: 'runSignInSaturation',
    },
    jwks_and_health_sentinel: {
      executor: 'constant-arrival-rate',
      rate: 50,
      timeUnit: '1s',
      duration: '100s',
      preAllocatedVUs: 20,
      maxVUs: 60,
      exec: 'runFastPathSentinel',
    },
  },
  thresholds: {
    jwks_read_duration: ['p(95)<15', 'p(99)<25'],
    health_probe_duration: ['p(95)<15', 'p(99)<25'],
    fast_path_available: ['rate>0.999'],
    auth_signin_duration: ['p(95)<1500'],
  },
};

export function runSignInSaturation() {
  const payload = JSON.stringify({
    email: testEmail,
    password: testPassword,
  });
  const res = http.post(`${baseUrl}/v1/client/auth/sign-in`, payload, {
    headers: {
      'Content-Type': 'application/json',
      'X-Publishable-Key': publishableKey,
    },
    tags: { workload: 'argon2_signin' },
  });
  signinDuration.add(res.timings.duration);
  check(res, {
    'sign-in returned expected status': (r) =>
      r.status === 200 || r.status === 401 || r.status === 429,
  });
}

export function runFastPathSentinel() {
  const healthRes = http.get(`${baseUrl}/health`, {
    tags: { workload: 'health_probe' },
  });
  healthDuration.add(healthRes.timings.duration);

  const jwksRes = http.get(`${baseUrl}/.well-known/jwks.json`, {
    tags: { workload: 'jwks_probe' },
  });
  jwksDuration.add(jwksRes.timings.duration);

  const ok = check(healthRes, { 'health 200': (r) => r.status === 200 }) &&
    check(jwksRes, { 'jwks 200': (r) => r.status === 200 });
  controlPlaneAvailability.add(ok);
  sleep(0.01);
}

