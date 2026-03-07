import type { DetailLayoutProps } from "@/layouts/detail/types";
import { resolveDetailLayoutType } from "@/layouts/detail/types";
import { CodeDetailLayout } from "@/layouts/detail/CodeDetailLayout";
import { MonoDetailLayout } from "@/layouts/detail/MonoDetailLayout";
import { MovieDetailLayout } from "@/layouts/detail/MovieDetailLayout";
import { WriteDetailLayout } from "@/layouts/detail/WriteDetailLayout";
import type { Project } from "@/store/orc-store";

type ProviderProps = DetailLayoutProps & {
  selectedProject: Project | null;
};

export function DetailLayoutProvider({ selectedProject, ...props }: ProviderProps) {
  const layoutType = resolveDetailLayoutType(props.detail, selectedProject);

  if (layoutType === "mono") {
    return <MonoDetailLayout {...props} />;
  }
  if (layoutType === "movie") {
    return <MovieDetailLayout {...props} />;
  }
  if (layoutType === "write") {
    return <WriteDetailLayout {...props} />;
  }
  return <CodeDetailLayout {...props} />;
}
