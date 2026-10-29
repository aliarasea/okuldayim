import { ApiClient } from '../client';
import { createEndpoints } from '../endpoints';
import type { Transport, TransportRequest } from '../transport';

describe('pickup endpoints', () => {
  it('attaches the idempotency key when creating a pickup request', async () => {
    const requests: TransportRequest[] = [];
    const transport: Transport = {
      send: async (request) => {
        requests.push(request);
        return { status: 201, body: {} };
      },
    };
    const tokens = {
      getValidAccessToken: async () => 'access-token',
      refresh: async () => null,
    };
    const endpoints = createEndpoints(new ApiClient(transport, tokens));

    await endpoints.pickup.create('child-1', 'person-1', 10, 'request-key-1');

    expect(requests[0]).toMatchObject({
      method: 'POST',
      path: '/v1/pickup-requests',
      body: { childId: 'child-1', pickupPersonId: 'person-1', etaMinutes: 10 },
      headers: { Authorization: 'Bearer access-token', 'Idempotency-Key': 'request-key-1' },
    });
  });
});
