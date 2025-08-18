import { MessageCircle, Settings, History } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface NavigationProps {
  activeTab: "chat" | "settings" | "history";
  onTabChange: (tab: "chat" | "settings" | "history") => void;
}

export function Navigation({ activeTab, onTabChange }: NavigationProps) {
  const tabs = [
    { id: "chat" as const, icon: MessageCircle, label: "Voice Chat" },
    { id: "history" as const, icon: History, label: "History" },
    { id: "settings" as const, icon: Settings, label: "Settings" },
  ];

  return (
    <nav className="flex gap-2 p-4 bg-card/50 backdrop-blur-sm border-b border-border/50">
      {tabs.map((tab) => (
        <Button
          key={tab.id}
          variant={activeTab === tab.id ? "default" : "ghost"}
          size="sm"
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "gap-2 transition-all duration-300 hover:shadow-md",
            activeTab === tab.id
              ? "bg-gradient-primary text-primary-foreground shadow-md"
              : "hover:bg-muted/80"
          )}
        >
          <tab.icon className="h-4 w-4" />
          {tab.label}
        </Button>
      ))}
    </nav>
  );
}