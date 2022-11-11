import Head from "next/head";

export default function Home() {
  return (
    <div className="container">
      <Head>
        <title>Cat Facts - NextJS</title>
        <link rel="icon" href="/favicon.ico" />
        <link
          rel="stylesheet"
          href="https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css"
        />
      </Head>

      <main>
        <section class="section title has-text-centered">
          <p>view.platform</p>
        </section>
        <section class="section container has-text-centered">
          <img src={"image.file.clone()"} style="height: 400px" />
        </section>
        <section class="section container has-text-centered">
          <p>{"view.fact"}</p>
        </section>
        <div class="buttons container is-centered">
          <button
            class="button is-primary is-danger"
            onclick={"link.callback(|_| CoreMessage::Message(Msg::Clear))"}
          >
            {"Clear"}
          </button>
          <button
            class="button is-primary is-success"
            onclick={"link.callback(|_| CoreMessage::Message(Msg::Get))"}
          >
            {"Get"}
          </button>
          <button
            class="button is-primary is-warning"
            onclick={"link.callback(|_| CoreMessage::Message(Msg::Fetch))"}
          >
            {"Fetch"}
          </button>
        </div>
      </main>
    </div>
  );
}
