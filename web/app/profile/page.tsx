import { redirect } from "next/navigation";

/** /profile is superseded by /dashboard */
export default function ProfilePage() {
  redirect("/dashboard");
}
