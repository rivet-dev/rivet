"use client";

import { Card, CardDescription, CardHeader, CardTitle } from "./ui/card";
import Link from "next/link";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Trash, ExternalLink, MoreVertical } from "lucide-react";
import { useRouter } from "next/navigation";
import { client } from "@/rivet/client";
import { toast } from "sonner";

type AppCardProps = {
  id: string;
  name: string;
  createdAt: Date;
  onDelete?: () => void;
};

export function AppCard({ id, name, createdAt, onDelete }: AppCardProps) {
  const router = useRouter();

  const handleOpen = (e: React.MouseEvent) => {
    e.preventDefault();
    router.push(`/app/${id}`);
  };

  const handleDelete = async (e: React.MouseEvent) => {
    e.preventDefault();

    try {
      await client.appStore.getOrCreate([id]).deleteApp();
      await client.appList.getOrCreate(["global"]).removeApp(id);
      toast.success("App deleted successfully");
      if (onDelete) {
        onDelete();
      }
    } catch (error) {
      console.error("Failed to delete app:", error);
      toast.error("Failed to delete app");
    }
  };

  return (
    <Card className="p-3 sm:p-4 border-b border rounded-md h-32 sm:h-36 relative w-full">
      <Link href={`/app/${id}`} className="cursor-pointer block">
        <CardHeader className="p-0">
          <CardTitle className="text-sm sm:text-base truncate">
            {name}
          </CardTitle>
          <CardDescription className="text-xs sm:text-sm">
            Created {createdAt.toLocaleDateString()}
          </CardDescription>
        </CardHeader>
      </Link>

      <div className="absolute top-2 right-2 transition-opacity">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 focus:outline-none">
              <MoreVertical className="h-4 w-4" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={handleOpen}>
              <ExternalLink className="mr-2 h-4 w-4" />
              Open
            </DropdownMenuItem>

            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={handleDelete}
              className="text-red-600 dark:text-red-400"
            >
              <Trash className="mr-2 h-4 w-4" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </Card>
  );
}
