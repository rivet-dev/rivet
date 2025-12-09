"use client";

import { Button } from "@/components/ui/button";
import {
  Share2Icon,
  LinkIcon,
  CopyIcon,
  ExternalLinkIcon,
} from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";

interface ShareButtonProps {
  className?: string;
  domain?: string;
  appId: string;
}

export function ShareButton({ className, domain }: ShareButtonProps) {
  const copyToClipboard = (text: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        toast.success("Link copied to clipboard!");
      })
      .catch(() => {
        toast.error("Failed to copy link");
      });
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={`flex items-center gap-1 ${className || ""}`}
        >
          Share
          <Share2Icon className="h-4 w-4 ml-1" />
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md w-[95vw] max-w-[450px]">
        <DialogHeader>
          <DialogTitle>Share App</DialogTitle>
          <DialogDescription>
            {domain
              ? "Share your app using the preview domain."
              : "No preview domain available yet."}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col space-y-6 mt-4">
          {domain ? (
            <>
              <div>
                <Label htmlFor="share-url" className="mb-2 block">
                  Preview Domain
                </Label>
                <div className="grid grid-cols-[1fr_auto] w-full overflow-hidden border border-input rounded-md">
                  <div className="overflow-hidden flex items-center bg-muted px-3 py-2">
                    <LinkIcon className="h-4 w-4 mr-2 flex-shrink-0 text-muted-foreground" />
                    <div className="truncate">
                      <span className="text-sm">https://{domain}</span>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    className="h-10 px-3 border-l border-input rounded-l-none"
                    onClick={() => copyToClipboard(`https://${domain}`)}
                  >
                    <CopyIcon className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              <div className="flex flex-col space-y-2 w-full">
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-2 w-full"
                  onClick={() => window.open(`https://${domain}`, "_blank")}
                >
                  <ExternalLinkIcon className="h-4 w-4" />
                  Visit Preview
                </Button>
              </div>
            </>
          ) : (
            <div className="text-center py-4">
              <p className="text-muted-foreground">
                No preview domain available yet.
              </p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
