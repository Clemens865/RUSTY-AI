import { useState } from "react";
import { Mic, MicOff, Play, Pause, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { VoiceVisualizer } from "./VoiceVisualizer";
import { cn } from "@/lib/utils";

export function VoiceChat() {
  const [isRecording, setIsRecording] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentMessage, setCurrentMessage] = useState("");

  const toggleRecording = () => {
    setIsRecording(!isRecording);
    if (!isRecording) {
      setCurrentMessage("Listening...");
    } else {
      setCurrentMessage("Processing your message...");
    }
  };

  const togglePlayback = () => {
    setIsPlaying(!isPlaying);
  };

  const stopAll = () => {
    setIsRecording(false);
    setIsPlaying(false);
    setCurrentMessage("");
  };

  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] p-8 animate-fade-in">
      {/* Main Voice Control */}
      <Card className="relative p-12 bg-gradient-bg border-border/50 shadow-lg hover:shadow-xl transition-all duration-500">
        {/* Pulsing Ring Animation */}
        {isRecording && (
          <div className="absolute inset-0 rounded-lg border-2 border-primary/30 animate-pulse-ring pointer-events-none" />
        )}
        
        <div className="flex flex-col items-center gap-8">
          {/* Voice Visualizer */}
          <VoiceVisualizer isActive={isRecording || isPlaying} />
          
          {/* Status Message */}
          <div className="text-center min-h-[60px] flex items-center">
            {currentMessage ? (
              <p className="text-lg text-muted-foreground animate-slide-up">
                {currentMessage}
              </p>
            ) : (
              <p className="text-xl font-medium text-foreground">
                Ready to chat
              </p>
            )}
          </div>
          
          {/* Main Control Button */}
          <Button
            onClick={toggleRecording}
            size="lg"
            className={cn(
              "h-24 w-24 rounded-full transition-all duration-300 transform hover:scale-105",
              isRecording
                ? "bg-gradient-secondary text-secondary-foreground shadow-lg animate-pulse"
                : "bg-gradient-primary text-primary-foreground shadow-md hover:shadow-lg"
            )}
          >
            {isRecording ? (
              <MicOff className="h-8 w-8" />
            ) : (
              <Mic className="h-8 w-8" />
            )}
          </Button>
          
          {/* Secondary Controls */}
          <div className="flex gap-4">
            <Button
              onClick={togglePlayback}
              variant="outline"
              size="icon"
              className="h-12 w-12 rounded-full border-border/50 hover:border-primary transition-all duration-300 hover:shadow-md"
            >
              {isPlaying ? (
                <Pause className="h-5 w-5" />
              ) : (
                <Play className="h-5 w-5" />
              )}
            </Button>
            
            <Button
              onClick={stopAll}
              variant="outline"
              size="icon"
              className="h-12 w-12 rounded-full border-border/50 hover:border-destructive hover:text-destructive transition-all duration-300 hover:shadow-md"
            >
              <Square className="h-5 w-5" />
            </Button>
          </div>
        </div>
      </Card>
      
      {/* Quick Actions */}
      <div className="mt-8 text-center animate-slide-up" style={{ animationDelay: "0.2s" }}>
        <p className="text-sm text-muted-foreground mb-4">
          Click the microphone to start a voice conversation
        </p>
        <div className="flex gap-2 justify-center">
          <span className="px-3 py-1 bg-muted rounded-full text-xs text-muted-foreground">
            Ask questions
          </span>
          <span className="px-3 py-1 bg-muted rounded-full text-xs text-muted-foreground">
            Get answers
          </span>
          <span className="px-3 py-1 bg-muted rounded-full text-xs text-muted-foreground">
            Natural conversation
          </span>
        </div>
      </div>
    </div>
  );
}