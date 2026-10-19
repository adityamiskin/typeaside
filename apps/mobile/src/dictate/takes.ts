import { File, Paths } from "expo-file-system";
import { z } from "zod";
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

const take = z.object({
  id: z.string(),
  createdAt: z.number(),
  text: z.string(),
  // Set until the audio is transcribed; `error` explains why the last attempt failed.
  pending: z.object({ uri: z.string(), error: z.string().optional() }).optional(),
  noteId: z.string().optional(),
});
export type Take = z.infer<typeof take>;

const MAX_TAKES = 200;
const file = new File(Paths.document, "takes.json");

export const useTakes = create<{
  takes: Take[];
  add: (take: Take) => void;
  update: (id: string, changes: Partial<Take>) => void;
  remove: (id: string) => void;
}>()(
  persist(
    (set) => ({
      takes: [],
      add: (take) => set(({ takes }) => ({ takes: [take, ...takes].slice(0, MAX_TAKES) })),
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
        setItem: (_, value) => file.write(value),
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
