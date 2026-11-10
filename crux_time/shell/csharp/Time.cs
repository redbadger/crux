using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_time</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// The shape of each method is the shape of the matching method of the
/// generated <c>IEffectHandler</c>. Thus an app's handler can hold an
/// <c>ITimeHandler</c> and delegate each operation to it in one line.
/// </para>
/// <para>
/// The shell answers <c>NotifyAt</c> and <c>NotifyAfter</c> exactly once, with
/// the id of the timer, when the timer fires. <c>NotifyEvery</c> is a stream:
/// the shell sends the id of the timer into the sink once per period.
/// <c>Clear</c> cancels the timer that it names, and answers with the same id.
/// </para>
/// <para>
/// The pending <c>NotifyAt</c> or <c>NotifyAfter</c> of a cleared timer can
/// still answer, so a shell does not need to race the two. The core no longer
/// listens for that answer, and ignores it. An implementation must still
/// complete the pending task, so that the task does not run for the life of the
/// process.
/// </para>
/// <para>
/// A repeating timer stops only when a <c>Clear</c> for its id arrives. A tick
/// that started before the <c>Clear</c> can still reach the core after it, and
/// the core ignores that tick. If the app aborts the command without a
/// <c>Clear</c>, the shell is not told. The timer then continues, and the core
/// ignores its ticks.
/// </para>
/// </remarks>
public interface ITimeHandler
{
    /// <summary>The current wall-clock time.</summary>
    Task<Instant> Now(Now operation);

    /// <summary>Answer with <c>operation.Id</c> once <c>operation.Instant</c> has arrived.</summary>
    Task<TimerId> NotifyAt(NotifyAt operation);

    /// <summary>Answer with <c>operation.Id</c> once <c>operation.Duration</c> has elapsed.</summary>
    Task<TimerId> NotifyAfter(NotifyAfter operation);

    /// <summary>
    /// Send <c>operation.Id</c> into <paramref name="sink"/> once per
    /// <c>operation.Period</c>, until a <c>Clear</c> for that id arrives.
    /// </summary>
    void NotifyEvery(NotifyEvery operation, IEffectSink<TimerId> sink);

    /// <summary>Cancel the timer <c>operation.Id</c> names, and answer with it.</summary>
    Task<TimerId> Clear(ClearTimer operation);
}
