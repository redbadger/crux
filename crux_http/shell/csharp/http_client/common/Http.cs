using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;

/// <summary>
/// The shell side of <c>crux_http</c>, as one method per operation.
/// </summary>
/// <remarks>
/// <para>
/// The method's shape is the generated <c>IEffectHandler</c>'s, so an app's
/// handler holds an instance and delegates in one line:
/// </para>
/// <code>
/// public sealed class MyHandler : IEffectHandler
/// {
///     private readonly IHttpHandler _http = new HttpClientHttpHandler();
///
///     public Task&lt;HttpResult&gt; Http(HttpRequest operation) => _http.Request(operation);
/// }
/// </code>
/// <para>
/// Implement it yourself to replace the implementation wholesale, in the app or
/// in a test.
/// </para>
/// </remarks>
public interface IHttpHandler
{
    /// <summary>Perform <paramref name="operation"/> and answer with its result, exactly once.</summary>
    Task<HttpResult> Request(HttpRequest operation);
}

/// <summary>
/// An <see cref="IHttpHandler"/> over <c>HttpClient</c>.
/// </summary>
/// <remarks>
/// <para>
/// Every exchange the client completes is an <c>HttpResult.Ok</c>, whatever the
/// status: 4xx and 5xx are answers, and the core decides what to make of them.
/// Only a transport failure — the request never reached a server, or no
/// response came back — is an <c>HttpResult.Err</c>, and then a URL that will
/// not parse is <c>HttpError.Url</c>, a cancellation or timeout is
/// <c>HttpError.Timeout</c>, and everything else is <c>HttpError.Io</c>.
/// </para>
/// <para>
/// The core builds <c>operation.Url</c> with <c>url::Url</c>, so the URL is
/// absolute. A relative URL, or a URL whose scheme is not <c>http</c> or
/// <c>https</c>, is <c>HttpError.Url</c>. A <c>GET</c> or <c>HEAD</c> with a
/// body is <c>HttpError.Io</c>, and the handler does not send it.
/// </para>
/// <para>
/// The client follows redirects by the rules of its handler, and the core gets
/// the last response. <c>SocketsHttpHandler</c>, the default on most
/// platforms, follows at most 50 redirects and does not follow a redirect from
/// <c>https</c> to <c>http</c>. To change this, pass a client of your own with
/// <c>AllowAutoRedirect</c> set on its handler.
/// </para>
/// <para>
/// <b>Cookies.</b> The default client keeps cookies, because the
/// <c>CookieContainer</c> of its <c>HttpClientHandler</c> is on by default
/// (<c>UseCookies</c>). That client is shared, so its cookies are
/// process-wide. A client of your own uses the cookie settings of its handler.
/// </para>
/// </remarks>
public sealed class HttpClientHttpHandler : IHttpHandler
{
    /// <summary>
    /// The client every handler made with the parameterless constructor sends
    /// with. <c>HttpClient</c> is meant to be created once and reused, because
    /// each instance holds its own connection pool, so there is only one. It is
    /// created on first use, so a shell that always passes its own client never
    /// creates it.
    /// </summary>
    private static readonly Lazy<HttpClient> DefaultClient = new(() => new HttpClient());

    private readonly HttpClient _client;

    /// <summary>
    /// Sends every request with one <c>HttpClient</c> shared by every handler
    /// made this way.
    /// </summary>
    /// <remarks>
    /// That client lives for the life of the process, so its connections and
    /// its cookies are process-wide. A long-running process that needs to see
    /// DNS changes should pass a client of its own instead, for example, where
    /// <c>SocketsHttpHandler</c> is available (it is not on Blazor WebAssembly),
    /// one over <c>new SocketsHttpHandler { PooledConnectionLifetime = TimeSpan.FromMinutes(2) }</c>.
    /// </remarks>
    public HttpClientHttpHandler()
        : this(DefaultClient.Value)
    {
    }

    /// <summary>
    /// Sends every request with <paramref name="client"/>, so that a handler
    /// chain, a proxy or a timeout of the shell's own choosing applies.
    /// </summary>
    public HttpClientHttpHandler(HttpClient client)
    {
        _client = client;
    }

    public async Task<HttpResult> Request(HttpRequest operation)
    {
        // `TryCreate` also accepts `file:` and `ftp:`, which a relative
        // `/api/x` parses to on Unix and which `HttpClient` cannot send.
        if (!Uri.TryCreate(operation.Url, UriKind.Absolute, out var uri)
            || (uri.Scheme != Uri.UriSchemeHttp && uri.Scheme != Uri.UriSchemeHttps))
        {
            return new HttpResult.Err(new HttpError.Url($"could not parse URL: {operation.Url}"));
        }

        var name = operation.Method.ToUpperInvariant();
        if (operation.Body.Length > 0 && (name is "GET" or "HEAD"))
        {
            return new HttpResult.Err(new HttpError.Io($"{name} request cannot carry a body"));
        }

        HttpMethod method;
        try
        {
            method = new HttpMethod(operation.Method);
        }
        catch (Exception e) when (e is ArgumentException or FormatException)
        {
            // An empty or malformed method is a request that cannot be made.
            return new HttpResult.Err(new HttpError.Io($"invalid HTTP method: {operation.Method}"));
        }

        using var message = new HttpRequestMessage(method, uri);
        // An empty body is no body: a GET with empty content is not the same
        // request.
        if (operation.Body.Length > 0)
        {
            message.Content = new ByteArrayContent(operation.Body);
        }

        // Header names and values cross the boundary as plain strings, with no
        // validation, so they are added without it — and a content header
        // belongs on the content, which is the only place it is accepted.
        foreach (var header in operation.Headers)
        {
            if (!message.Headers.TryAddWithoutValidation(header.Name, header.Value))
            {
                message.Content ??= new ByteArrayContent(Array.Empty<byte>());
                message.Content.Headers.TryAddWithoutValidation(header.Name, header.Value);
            }
        }

        try
        {
            using var response = await _client.SendAsync(message).ConfigureAwait(false);

            var headers = new ObservableCollection<HttpHeader>();
            foreach (var header in Flatten(response.Headers))
            {
                headers.Add(header);
            }
            foreach (var header in Flatten(response.Content.Headers))
            {
                headers.Add(header);
            }

            var body = await response.Content.ReadAsByteArrayAsync().ConfigureAwait(false);

            return new HttpResult.Ok(new HttpResponse
            {
                Status = (ushort)response.StatusCode,
                Headers = headers,
                Body = body,
            });
        }
        catch (OperationCanceledException)
        {
            // `HttpClient` reports its own timeout as a cancellation.
            return new HttpResult.Err(new HttpError.Timeout());
        }
        catch (HttpRequestException e)
        {
            return new HttpResult.Err(new HttpError.Io(e.Message));
        }
        catch (Exception e) when (e is InvalidOperationException or NotSupportedException)
        {
            // Anything else the client refuses to send is still an answer.
            return new HttpResult.Err(new HttpError.Io(e.Message));
        }
    }

    private static IEnumerable<HttpHeader> Flatten(
        IEnumerable<KeyValuePair<string, IEnumerable<string>>> headers)
    {
        foreach (var (name, values) in headers)
        {
            foreach (var value in values)
            {
                yield return new HttpHeader { Name = name, Value = value };
            }
        }
    }
}
