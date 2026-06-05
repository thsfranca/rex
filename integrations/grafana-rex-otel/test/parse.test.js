const test = require('node:test');
const assert = require('node:assert/strict');

// Mirror otelToFrames logic for CI without Grafana SDK build.
function otelToFrames(payload, refId) {
  const frames = [];
  for (const rm of payload.resourceMetrics ?? []) {
    for (const sm of rm.scopeMetrics ?? []) {
      for (const metric of sm.metrics ?? []) {
        const points = metric.sum?.dataPoints ?? metric.histogram?.dataPoints ?? [];
        const times = [];
        const values = [];
        for (const dp of points) {
          times.push(Number(dp.timeUnixNano) / 1e6);
          values.push(dp.asInt != null ? Number(dp.asInt) : Number(dp.sum ?? 0));
        }
        frames.push({ refId, name: metric.name, times, values });
      }
    }
  }
  return frames;
}

test('otelToFrames maps counter point', () => {
  const payload = {
    resourceMetrics: [
      {
        scopeMetrics: [
          {
            metrics: [
              {
                name: 'rex.stream.requests',
                sum: {
                  dataPoints: [{ timeUnixNano: '1000000000', asInt: '1' }],
                },
              },
            ],
          },
        ],
      },
    ],
  };
  const frames = otelToFrames(payload, 'A');
  assert.equal(frames.length, 1);
  assert.equal(frames[0].name, 'rex.stream.requests');
  assert.deepEqual(frames[0].values, [1]);
});
