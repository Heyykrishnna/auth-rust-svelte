import http from 'k6/http';
import { check } from 'k6';
import { Trend, Counter, Rate } from 'k6/metrics';

const baseUrl = __ENV.AUTH_API_URL || 'http://127.0.0.1:8080';
const backendSecretKey = __ENV.BACKEND_SECRET_KEY || '';
const targetAppUserId = __ENV.TARGET_APP_USER_ID || '';

const mutationLatency = new Trend('webhook_burst_mutation_duration', true);
const syncCatchupLatency = new Trend('backend_sync_catchup_duration', true);
const eventsProduced = new Counter('outbox_events_triggered');
const burstSuccess = new Rate('webhook_burst_success');

export const options = {
  scenarios: {
    outbox_fanout_burst: {
      executor: 'ramping-arrival-rate',
      startRate: 10,
      timeUnit: '1s',
      preAllocatedVUs: 30,
      maxVUs: 120,
      stages: [
        { duration: '15s', target: 50 },
        { duration: '60s', target: 150 },
        { duration: '15s', target: 20 },
      ],
      exec: 'runMetadataMutationBurst',
    },
    reconciliation_catchup_reader: {
      executor: 'constant-arrival-rate',
      rate: 10,
      timeUnit: '1s',
      duration: '90s',
      preAllocatedVUs: 10,
      maxVUs: 30,
      exec: 'runEventsCatchup',
    },
  },
  thresholds: {
    webhook_burst_mutation_duration: ['p(95)<120', 'p(99)<250'],
    backend_sync_catchup_duration: ['p(95)<80', 'p(99)<150'],
    webhook_burst_success: ['rate>0.99'],
  },
};

export function runMetadataMutationBurst() {
  if (!backendSecretKey || !targetAppUserId) {
    const ping = http.get(`${baseUrl}/health`, {
      tags: { workload: 'burst_dry_run' },
    });
    mutationLatency.add(ping.timings.duration);
    burstSuccess.add(ping.status === 200);
    return;
  }

  const payload = JSON.stringify({
    public_metadata: {
      load_burst_seq: `${__VU}-${__ITER}`,
      ts: Date.now(),
    },
  });

  const res = http.patch(
    `${baseUrl}/v1/backend/users/${targetAppUserId}/metadata`,
    payload,
    {
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${backendSecretKey}`,
      },
      tags: { workload: 'outbox_mutation_burst' },
    },
  );
  mutationLatency.add(res.timings.duration);
  const ok = check(res, {
    'metadata update accepted': (r) => r.status === 200,
  });
  if (ok) {
    eventsProduced.add(1);
  }
  burstSuccess.add(ok);
}

export function runEventsCatchup() {
  if (!backendSecretKey) {
    const health = http.get(`${baseUrl}/health`, {
      tags: { workload: 'catchup_dry_run' },
    });
    syncCatchupLatency.add(health.timings.duration);
    burstSuccess.add(health.status === 200);
    return;
  }

  const res = http.get(`${baseUrl}/v1/backend/events?limit=50`, {
    headers: {
      Authorization: `Bearer ${backendSecretKey}`,
    },
    tags: { workload: 'events_catchup' },
  });
  syncCatchupLatency.add(res.timings.duration);
  const ok = check(res, {
    'events catchup 200': (r) => r.status === 200,
  });
  burstSuccess.add(ok);
}

