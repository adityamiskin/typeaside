import { createFileRoute } from "@tanstack/react-router";
import { SignIn } from "../components/site";
export const Route = createFileRoute("/sign-in")({ component: () => <SignIn /> });
