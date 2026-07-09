// SPDX-License-Identifier: MIT

/** Serializes async work so concurrent calls run one after another. */
export function createAsyncQueue<T = void>(): {
  enqueue<R>(fn: () => Promise<R>): Promise<R>
} {
  let chain: Promise<T> = Promise.resolve(undefined as T)

  return {
    enqueue<R>(fn: () => Promise<R>): Promise<R> {
      const run = chain.then(fn) as Promise<R>
      chain = run.then(
        () => undefined as T,
        () => undefined as T,
      ) as Promise<T>
      return run
    },
  }
}

/** Like createAsyncQueue, but the chain always resolves to boolean success. */
export function createSaveQueue(): {
  enqueue(fn: () => Promise<boolean>): Promise<boolean>
} {
  let chain: Promise<boolean> = Promise.resolve(true)

  return {
    enqueue(fn: () => Promise<boolean>): Promise<boolean> {
      const run = chain.then(fn)
      chain = run.then(
        () => true,
        () => false,
      )
      return run
    },
  }
}
