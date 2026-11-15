import { createApp } from "./app.js";
import { env, hasOpenAiKey } from "./config/env.js";

const app = createApp();

app.listen(env.port, () => {
  console.log(`Study Buddy AI backend listening on http://localhost:${env.port}`);
  if (!hasOpenAiKey()) {
    console.warn(
      "⚠️  OPENAI_API_KEY is not set — notes will use basic local cleanup instead of AI rewrite. " +
        "Set it in backend/.env to enable the concise rewrite.",
    );
  }
});
