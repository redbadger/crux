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
///     private readonly IHttpHandler _http = HttpClientHttpHandler.Shared;
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
/// </remarks>
public sealed class HttpClientHttpHandler : IHttpHandler
{
    /// <summary>
    /// The plain path: one handler over one <c>HttpClient</c>, which is how
    /// <c>HttpClient</c> wants to be used — one instance, reused.
    /// </summary>
    public static readonly HttpClientHttpHandler Shared = new(new HttpClient());

    private readonly HttpClient _client;

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
        if (!Uri.TryCreate(operation.Url, UriKind.Absolute, out var uri))
        {
            return new HttpResult.Err(new HttpError.Url($"could not parse URL: {operation.Url}"));
        }

        using var message = new HttpRequestMessage(new HttpMethod(operation.Method), uri);
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
