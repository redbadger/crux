/**
 * The shell side of `crux_time`, as one method per operation.
 *
 * The shape of each method is the shape of the matching method of the
 * generated `EffectHandler`. Thus an app's handler can hold a `TimeHandler`
 * and delegate each operation to it in one line.
 *
 * ## What a timer answers with, and when
 *
 * The shell answers `notifyAt` and `notifyAfter` exactly once, with the id of
 * the timer, when the timer fires. `clear` cancels the timer that it names, and
 * answers with the same id.
 *
 * The pending `notifyAt` or `notifyAfter` of a cleared timer can still answer,
 * so a shell does not need to race the two. The core no longer listens for that
 * answer, and ignores it. An implementation must still settle the pending
 * promise, so that the promise does not stay unresolved for the life of the
 * page.
 */
export interface TimeHandler {
  /** The current wall-clock time. */
  now(operation: Now): Promise<Instant>;
  /** Answer with `operation.id` once `operation.instant` has arrived. */
  notifyAt(operation: NotifyAt): Promise<TimerId>;
  /** Answer with `operation.id` once `operation.duration` has elapsed. */
  notifyAfter(operation: NotifyAfter): Promise<TimerId>;
  /** Cancel the timer `operation.id` names, and answer with it. */
  clear(operation: ClearTimer): Promise<TimerId>;
}
