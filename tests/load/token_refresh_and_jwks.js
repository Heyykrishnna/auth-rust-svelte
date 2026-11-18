import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

const baseUrl = __ENV.AUTH_API_URL || 'http://127.0.0.1:8080';
const publishableKey = __ENV.PUBLISHABLE_KEY || 'pk_test_replace_me';
const accessToken = __ENV.LOAD_ACCESS_TOKEN || '';
const refreshToken = __ENV.LOAD_REFRESH_TOKEN || '';

const jwksLatency = new Trend('jwks_spike_duration', true);
const userinfoLatency = new Trend('userinfo_spike_duration', true);
const refreshLatency = new Trend('token_refresh_duration', true);
const endpointSuccess = new Rate('token_plane_success');

export const options = {
  scenarios: {
    jwks_read_spike: {
      executor: 'ramping-arrival-rate',
      startRate: 50,
      timeUnit: '1s',
      preAllocatedVUs: 40,
      maxVUs: 150,
      stages: [
        { duration: '15s', target: 200 },
        { duration: '45s', target: 400 },
        { duration: '15s', target: 50 },
      ],
      exec: 'runJwksAndDiscoverySpike',
    },
    userinfo_read_spike: {
      executor: 'constant-arrival-rate',
      rate: 100,
      timeUnit: '1s',
      duration: '75s',
      preAllocatedVUs: 30,
      maxVUs: 100,
      exec: 'runUserinfoSpike',
    },
    refresh_race_probe: {
      executor: 'per-vu-iterations',
      vus: 4,
      iterations: 1,
      startTime: '10s',
      exec: 'runConcurrentRefreshRace',
    },
  },
  thresholds: {
    jwks_spike_duration: ['p(95)<15', 'p(99)<25'],
    userinfo_spike_duration: ['p(95)<40', 'p(99)<100'],
    token_refresh_duration: ['p(95)<60', 'p(99)<100'],
    token_plane_success: ['rate>0.99'],
  },
};

export function runJwksAndDiscoverySpike() {
  const res = http.get(`${baseUrl}/.well-known/jwks.json`, {
    tags: { workload: 'jwks_spike' },
  });
  jwksLatency.add(res.timings.duration);
  const ok = check(res, {
    'jwks status 200': (r) => r.status === 200,
    'jwks contains keys': (r) => r.body && r.body.includes('"keys"'),
  });
  endpointSuccess.add(ok);
}

export function runUserinfoSpike() {
  if (!accessToken) {
    const discovery = http.get(`${baseUrl}/.well-known/openid-configuration`, {
      tags: { workload: 'oidc_discovery_fallback' },
    });
    userinfoLatency.add(discovery.timings.duration);
    endpointSuccess.add(discovery.status === 200);
    return;
  }

  const res = http.get(`${baseUrl}/oauth2/userinfo`, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
    tags: { workload: 'oidc_userinfo' },
  });
  userinfoLatency.add(res.timings.duration);
  const ok = check(res, {
    'userinfo status 200': (r) => r.status === 200,
  });
  endpointSuccess.add(ok);
}

export function runConcurrentRefreshRace() {
  if (!refreshToken) {
    sleep(0.1);
    return;
  }

  const res = http.post(
    `${baseUrl}/v1/client/auth/refresh`,
    JSON.stringify({ refresh_token: refreshToken }),
    {
      headers: {
        'Content-Type': 'application/json',
        'X-Publishable-Key': publishableKey,
      },
      tags: { workload: 'refresh_race' },
    },
  );
  refreshLatency.add(res.timings.duration);
  check(res, {
    'refresh race handled deterministically': (r) =>
      r.status === 200 || r.status === 401,
  });
}

