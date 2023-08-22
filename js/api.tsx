import { Record, RecordOf } from "immutable";

export async function get(endpoint: string): Promise<Response> {
  return await fetch(endpoint, {
    headers: { Accept: "application/json" },
    method: "GET",
  });
}

export async function put(endpoint: string, body: BodyInit): Promise<Response> {
  return await fetch(endpoint, {
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    method: "PUT",
    body,
  });
}

export async function getJson<T>(endpoint: string): Promise<ApiResponse<T>> {
  return await convertResponse<T>(await get(endpoint));
}

export async function putJson<T>(
  endpoint: string,
  body: BodyInit
): Promise<ApiResponse<T>> {
  return await convertResponse(await put(endpoint, body));
}

export type ApiResponse<T> =
  | { is: "ok"; s: T }
  | { is: "error"; s: { error: Error } };

async function convertResponse<T>(response: Response): Promise<ApiResponse<T>> {
  if (response.ok) {
    return { is: "ok", s: await response.json() };
  }
  try {
    const err_body = await response.json();
    return { is: "error", s: err_body };
  } catch {
    return {
      is: "error",
      s: {
        error: {
          code: response.status,
          description: response.statusText,
          reason: "unknown",
        },
      },
    };
  }
}

export type BuildStatus =
  | "queued"
  | "building"
  | "uploading"
  | "succeeded"
  | "failed"
  | "canceled";

export const isRunning = (t: BuildStatus) => {
  switch (t) {
    case "canceled":
    case "failed":
    case "succeeded":
      return false;
    default:
      return true;
  }
};

export type TailEvent =
  | { t: "Text"; c: string }
  | { t: "Lines"; c: string[] }
  | { t: "Error"; c: string }
  | { t: "Reset" };

export type Build = {
  id: number;
  origin: string;
  rev: string;
  created_at: string;
  status: BuildStatus;
  finished_at: string | null;
  error_msg: string | null;
};

type BuildNewProps = {
  origin: string;
  rev: string;
  paths: string;
};

export type BuildNew = RecordOf<BuildNewProps>;
export const BuildNew: Record.Factory<BuildNewProps> = Record({
  origin: "",
  rev: "main",
  paths: "",
});

export type Input = {
  id: number;
  build_id: number;
  path: string;
};

export type Output = {
  id: number;
  input_id: number;
  system: string;
  store_path: string;
};

export type InputOutputs = Input & {
  outputs: Output[];
};

export type GetBuild = {
  build: Build;
  inputs: InputOutputs[];
};

export type Error = {
  code: number;
  reason: string;
  description: string;
};
