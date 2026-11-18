import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

const baseUrl = __ENV.AUTH_API_URL || 'http://127.0.0.1:8080';
const soakDuration = __ENV.SOAK_DURATION || '30m';
const publishableKey = __ENV.PUBLISHABLE_KEY || 'pk_test_replace_me';
const backendSecretKey = __ENV.BACKEND_SECRET_KEY || '';

const fastPathLatency = new Trend('soak_fast_path_duration', true);
const configPathLatency = new Trend('soak_config_duration', true);
const soakErrorFree = new Rate('soak_success_rate');

export const options = {
  scenarios: {
    sustained_jwks_and_discovery: {
      executor: 'constant-arrival-rate',
      rate: 40,
      timeUnit: '1s',
      duration: soakDuration,
      preAllocatedVUs: 20,
      maxVUs: 60,
      exec: 'runFastReadLoop',
    },
    sustained_tenant_config_and_sync: {
      executor: 'constant-arrival-rate',
      rate: 10,
      timeUnit: '1s',
      duration: soakDuration,
      preAllocatedVUs: 10,
      maxVUs: 30,
      exec: 'runTenantAndSyncLoop',
    },
  },
  thresholds: {
    soak_fast_path_duration: ['p(95)<15', 'p(99)<25'],
    soak_config_duration: ['p(95)<50', 'p(99)<100'],
    soak_success_rate: ['rate>0.999'],
    http_req_failed: ['rate<0.001'],
  },
};

export function runFastReadLoop() {
  const jwks = http.get(`${baseUrl}/.well-known/jwks.json`, {
    tags: { workload: 'soak_jwks' },
  });
  fastPathLatency.add(jwks.timings.duration);

  const discovery = http.get(`${baseUrl}/.well-known/openid-configuration`, {
    tags: { workload: 'soak_discovery' },
  });
  fastPathLatency.add(discovery.timings.duration);

  const ok = check(jwks, { 'jwks 200': (r) => r.status === 200 }) &&
    check(discovery, { 'discovery 200': (r) => r.status === 200 });
  soakErrorFree.add(ok);
}

export function runTenantAndSyncLoop() {
  const health = http.get(`${baseUrl}/health`, {
    tags: { workload: 'soak_health' },
  });
  configPathLatency.add(health.timings.duration);
  let ok = check(health, { 'health 200': (r) => r.status === 200 });

  if (publishableKey && publishableKey !== 'pk_test_replace_me') {
    const cfg = http.get(`${baseUrl}/v1/client/config`, {
      headers: { 'X-Publishable-Key': publishableKey },
      tags: { workload: 'soak_client_config' },
    });
    configPathLatency.add(cfg.timings.duration);
    ok = ok && check(cfg, { 'client config 200': (r) => r.status === 200 });
  }

  if (backendSecretKey) {
    const ping = http.get(`${baseUrl}/v1/backend/ping`, {
      headers: { Authorization: `Bearer ${backendSecretKey}` },
      tags: { workload: 'soak_backend_ping' },
    });
    configPathLatency.add(ping.timings.duration);
    ok = ok && check(ping, { 'backend ping 200': (r) => r.status === 200 });
  }

  soakErrorFree.add(ok);
  sleep(0.05);
}

