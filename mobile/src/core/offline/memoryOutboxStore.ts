import type { OutboxCounts, OutboxItem, OutboxStore } from './outbox';

/** Testler için bellek içi depo. */
export class MemoryOutboxStore implements OutboxStore {
  readonly items = new Map<string, OutboxItem>();

  async insert(item: OutboxItem): Promise<void> {
    this.items.set(item.id, { ...item });
  }

  async listPending(): Promise<OutboxItem[]> {
    return [...this.items.values()]
      .filter((item) => item.state === 'PENDING')
      .sort((a, b) => a.createdAt - b.createdAt)
      .map((item) => ({ ...item }));
  }

  async update(item: OutboxItem): Promise<void> {
    this.items.set(item.id, { ...item });
  }

  async remove(id: string): Promise<void> {
    this.items.delete(id);
  }

  async counts(): Promise<OutboxCounts> {
    const all = [...this.items.values()];
    return {
      pending: all.filter((item) => item.state === 'PENDING').length,
      failed: all.filter((item) => item.state === 'FAILED').length,
    };
  }
}
