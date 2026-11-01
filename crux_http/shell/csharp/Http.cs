using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_http</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// The shape of the method is the shape of the matching method of the
/// generated <c>IEffectHandler</c>. Thus an app's handler can hold an
/// <c>IHttpHandler</c> and delegate the operation to it in one line.
/// </para>
/// <para>
/// Every exchange that completes is an <c>HttpResult.Ok</c>, whatever the
/// status. A 4xx or 5xx status is an answer, and the core decides what to do
/// with it. Only a transport failure is an <c>HttpResult.Err</c>. That is, the
/// request did not reach a server, or no response came back. Then a URL that
/// does not parse is <c>HttpError.Url</c>, a cancellation or a timeout is
/// <c>HttpError.Timeout</c>, and every other transport failure is
/// <c>HttpError.Io</c>.
/// </para>
/// <para>
/// The core builds <c>operation.Url</c> with <c>url::Url</c>, so the URL is
/// absolute. A relative URL is <c>HttpError.Url</c>. A <c>GET</c> or
/// <c>HEAD</c> request with a body is <c>HttpError.Io</c>, and the handler does
/// not send it.
/// </para>
/// </remarks>
public interface IHttpHandler
{
    /// <summary>Perform <paramref name="operation"/> and answer with its result, exactly once.</summary>
    Task<HttpResult> Request(HttpRequest operation);
}
