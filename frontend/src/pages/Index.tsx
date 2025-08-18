import { useState } from "react";
import { Navigation } from "@/components/Navigation";
import { VoiceChat } from "@/components/VoiceChat";
import { Settings } from "@/components/Settings";
import { History } from "@/components/History";

const Index = () => {
  const [activeTab, setActiveTab] = useState<"chat" | "settings" | "history">("chat");

  const renderActiveTab = () => {
    switch (activeTab) {
      case "chat":
        return <VoiceChat />;
      case "settings":
        return <Settings />;
      case "history":
        return <History />;
      default:
        return <VoiceChat />;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-bg">
      <div className="max-w-6xl mx-auto">
        <Navigation activeTab={activeTab} onTabChange={setActiveTab} />
        <main className="animate-scale-in">
          {renderActiveTab()}
        </main>
      </div>
    </div>
  );
};

export default Index;
