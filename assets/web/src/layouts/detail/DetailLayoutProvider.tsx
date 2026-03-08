import type { DetailLayoutProps } from "@/layouts/detail/types";
import { resolveDetailLayoutType } from "@/layouts/detail/types";
import { CodeDetailLayout } from "@/layouts/detail/CodeDetailLayout";
import { MonoDetailLayout } from "@/layouts/detail/MonoDetailLayout";
import type { Project } from "@/store/orc-store";

type ProviderProps = DetailLayoutProps & {
  selectedProject: Project | null;
};

export function DetailLayoutProvider({ selectedProject, ...props }: ProviderProps) {
  const layoutType = resolveDetailLayoutType(props.detail, selectedProject);

  if (layoutType === "mono") {
    return <MonoDetailLayout {...props} />;
  }
  return <CodeDetailLayout {...props} />;
}
