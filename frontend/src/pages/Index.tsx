import { useState } from "react";
import { Navigation } from "@/components/Navigation";
import { VoiceChatEnhanced } from "@/components/VoiceChatEnhanced";
import { ChatInterface } from "@/components/ChatInterface";
import { KnowledgeBase } from "@/components/KnowledgeBase";
import { Settings } from "@/components/Settings";
import { History } from "@/components/History";

const Index = () => {
  const [activeTab, setActiveTab] = useState<"voice" | "chat" | "knowledge" | "settings" | "history">("voice");

  const renderActiveTab = () => {
    switch (activeTab) {
      case "voice":
        return <VoiceChatEnhanced />;
      case "chat":
        return <ChatInterface />;
      case "knowledge":
        return <KnowledgeBase />;
      case "settings":
        return <Settings />;
      case "history":
        return <History />;
      default:
        return <VoiceChatEnhanced />;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-bg">
      <div className="max-w-7xl mx-auto">
        <Navigation activeTab={activeTab} onTabChange={setActiveTab} />
        <main className="animate-scale-in">
          {renderActiveTab()}
        </main>
      </div>
    </div>
  );
};

export default Index;
