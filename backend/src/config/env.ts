import dotenv from "dotenv";

dotenv.config();

export const env = {
  port: Number(process.env.PORT) || 3001,
  openaiApiKey: process.env.OPENAI_API_KEY ?? "",
  openaiModel: process.env.OPENAI_MODEL || "gpt-4o",
  corsOrigin: process.env.CORS_ORIGIN || "http://localhost:5173",
};

export const hasOpenAiKey = (): boolean => env.openaiApiKey.trim().length > 0;
