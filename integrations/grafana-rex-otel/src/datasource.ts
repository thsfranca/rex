import { DataSourceInstanceSettings } from '@grafana/data';

export interface RexQuery {
  instrument: string;
  refId: string;
}

export interface RexDataSourceOptions {
  url: string;
  catalogPath?: string;
  queryPath?: string;
}

export class RexDataSource {
  constructor(private instanceSettings: DataSourceInstanceSettings<RexDataSourceOptions>) {}

  get baseUrl(): string {
    return this.instanceSettings.url.replace(/\/$/, '');
  }

  async testDatasource() {
    const resp = await fetch(`${this.baseUrl}/health`);
    if (!resp.ok) {
      return { status: 'error', message: `health ${resp.status}` };
    }
    return { status: 'success', message: 'Rex read API OK' };
  }

  async query(request: { targets: RexQuery[]; range: { from: number; to: number } }) {
    const queryPath = this.instanceSettings.jsonData.queryPath ?? '/v1/metrics/query';
    const frames = [];
    for (const target of request.targets) {
      const body = {
        start_ms: request.range.from,
        end_ms: request.range.to,
        instruments: [target.instrument],
        labels: {},
      };
      const resp = await fetch(`${this.baseUrl}${queryPath}`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!resp.ok) {
        throw new Error(`query failed: ${resp.status}`);
      }
      const payload = await resp.json();
      frames.push(...otelToFrames(payload, target.refId));
    }
    return { data: frames };
  }
}

export function otelToFrames(payload: any, refId: string) {
  const frames: any[] = [];
  const resourceMetrics = payload.resourceMetrics ?? [];
  for (const rm of resourceMetrics) {
    for (const sm of rm.scopeMetrics ?? []) {
      for (const metric of sm.metrics ?? []) {
        const points =
          metric.sum?.dataPoints ??
          metric.histogram?.dataPoints ??
          [];
        const times: number[] = [];
        const values: number[] = [];
        for (const dp of points) {
          const nano = Number(dp.timeUnixNano ?? 0);
          times.push(nano / 1e6);
          if (dp.asInt != null) {
            values.push(Number(dp.asInt));
          } else if (dp.sum != null) {
            values.push(Number(dp.sum));
          } else {
            values.push(0);
          }
        }
        frames.push({
          refId,
          name: metric.name,
          fields: [
            { name: 'Time', type: 'time', values: times },
            { name: 'Value', type: 'number', values: values },
          ],
        });
      }
    }
  }
  return frames;
}
