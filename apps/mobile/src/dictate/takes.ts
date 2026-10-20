import { File, Paths } from "expo-file-system";
import { z } from "zod";
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export const take = z.object({
  id: z.string(),
  createdAt: z.number(),
  text: z.string(),
  noteId: z.string().optional(),
});
export type Take = z.infer<typeof take>;

const MAX_TAKES = 200;
const file = new File(Paths.document, "takes.json");

export const useTakes = create<{
  takes: Take[];
  merge: (incoming: Take[]) => void;
  update: (id: string, changes: Partial<Take>) => void;
  remove: (id: string) => void;
}>()(
  persist(
    (set) => ({
      takes: [],
      merge: (incoming) =>
        set(({ takes }) => {
          const ids = new Set(takes.map((take) => take.id));
          return {
            takes: [...takes, ...incoming.filter((take) => !ids.has(take.id))]
              .sort((a, b) => b.createdAt - a.createdAt)
              .slice(0, MAX_TAKES),
          };
        }),
      update: (id, changes) =>
        set(({ takes }) => ({
          takes: takes.map((take) => (take.id === id ? { ...take, ...changes } : take)),
        })),
      remove: (id) => set(({ takes }) => ({ takes: takes.filter((take) => take.id !== id) })),
    }),
    {
      name: "takes",
      version: 1,
      storage: createJSONStorage(() => ({
        getItem: () => (file.exists ? file.textSync() : null),
        setItem: (_, value) => file.writeSync(value),
        removeItem: () => file.delete(),
      })),
      partialize: ({ takes }) => ({ takes }),
      merge: (persisted, current) => ({
        ...current,
        takes: z.object({ takes: z.array(take) }).safeParse(persisted).data?.takes ?? [],
      }),
    },
  ),
);
