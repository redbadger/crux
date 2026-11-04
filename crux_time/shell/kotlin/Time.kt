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
 * answer, and ignores it. An implementation must still finish the pending call,
 * so that the call does not stay suspended for the life of the process.
 */
interface TimeHandler {
    /** The current wall-clock time. */
    suspend fun now(operation: Now): Instant

    /** Answer with `operation.id` once `operation.instant` has arrived. */
    suspend fun notifyAt(operation: NotifyAt): TimerId

    /** Answer with `operation.id` once `operation.duration` has elapsed. */
    suspend fun notifyAfter(operation: NotifyAfter): TimerId

    /** Cancel the timer `operation.id` names, and answer with it. */
    suspend fun clear(operation: ClearTimer): TimerId
}
