import { cn } from "@/lib/utils";

interface VoiceVisualizerProps {
  isActive: boolean;
  className?: string;
}

export function VoiceVisualizer({ isActive, className }: VoiceVisualizerProps) {
  return (
    <div className={cn("flex items-center justify-center gap-1", className)}>
      {Array.from({ length: 5 }, (_, i) => (
        <div
          key={i}
          className={cn(
            "w-1 bg-gradient-primary rounded-full transition-all duration-300",
            isActive ? "animate-bounce-gentle" : "opacity-50",
            i === 0 || i === 4 ? "h-8" : i === 1 || i === 3 ? "h-12" : "h-16"
          )}
          style={{
            animationDelay: isActive ? `${i * 0.1}s` : "0s",
          }}
        />
      ))}
    </div>
  );
}