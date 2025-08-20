import { useState, useRef, useEffect } from "react";
import { Mic, MicOff, Send, Bot, User, Volume2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { VoiceVisualizer } from "./VoiceVisualizer";
import { API_CONFIG, API_ENDPOINTS, buildApiUrl } from "@/config/api-config";
import { cn } from "@/lib/utils";

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
  type: "voice" | "text";
}

export function VoiceChatEnhanced() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [currentStatus, setCurrentStatus] = useState("Ready to chat");
  
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const scrollAreaRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (scrollAreaRef.current) {
      scrollAreaRef.current.scrollTop = scrollAreaRef.current.scrollHeight;
    }
  }, [messages]);

  // Start recording audio
  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: 'audio/webm;codecs=opus'
      });
      
      mediaRecorderRef.current = mediaRecorder;
      audioChunksRef.current = [];
      
      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
        }
      };
      
      mediaRecorder.onstop = async () => {
        const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/webm' });
        await processAudioInput(audioBlob);
        
        // Stop all tracks to release microphone
        stream.getTracks().forEach(track => track.stop());
      };
      
      mediaRecorder.start();
      setIsRecording(true);
      setCurrentStatus("Listening...");
    } catch (error) {
      console.error("Error accessing microphone:", error);
      setCurrentStatus("Microphone access denied");
    }
  };

  // Stop recording
  const stopRecording = () => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
      setCurrentStatus("Processing your voice...");
    }
  };

  // Toggle recording
  const toggleRecording = () => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  };

  // Process audio input (convert to text and send to AI)
  const processAudioInput = async (audioBlob: Blob) => {
    setIsProcessing(true);
    setCurrentStatus("Transcribing your voice...");
    
    try {
      // Send audio to backend for transcription
      const formData = new FormData();
      formData.append('audio', audioBlob, 'recording.webm');
      
      const response = await fetch('/api/v1/voice/transcribe', {
        method: 'POST',
        body: formData,
      });
      
      if (!response.ok) {
        throw new Error('Transcription failed');
      }
      
      const { text } = await response.json();
      console.log("Transcribed text:", text);
      
      // Add user message
      const userMessage: Message = {
        id: crypto.randomUUID(),
        role: "user",
        content: text,
        timestamp: new Date(),
        type: "voice",
      };
      
      setMessages(prev => [...prev, userMessage]);
      
      // Send to AI backend
      await sendMessage(text, "voice");
    } catch (error) {
      console.error("Transcription error:", error);
      setCurrentStatus("Transcription failed. Please try again.");
    } finally {
      setIsProcessing(false);
      setCurrentStatus("Ready to chat");
    }
  };

  // Send message to backend
  const sendMessage = async (text: string, type: "voice" | "text" = "text") => {
    if (!text.trim()) return;
    
    setIsProcessing(true);
    
    try {
      const response = await fetch(API_ENDPOINTS.conversation.send, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          message: text,
          session_id: sessionId,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      
      // Store session ID for future messages
      if (data.session_id) {
        setSessionId(data.session_id);
      }

      const assistantMessage: Message = {
        id: crypto.randomUUID(),
        role: "assistant",
        content: data.response || "No response received",
        timestamp: new Date(),
        type: "text",
      };

      setMessages(prev => [...prev, assistantMessage]);
      
      // If it was a voice input, speak the response using TTS API
      if (type === "voice") {
        await speakResponseWithAPI(data.response);
      }
    } catch (error) {
      console.error("Error sending message:", error);
      setCurrentStatus("Error processing message");
    } finally {
      setIsProcessing(false);
      setCurrentStatus("Ready to chat");
    }
  };

  // Text-to-speech using backend API
  const speakResponseWithAPI = async (text: string) => {
    try {
      setIsPlaying(true);
      
      const response = await fetch('/api/v1/voice/synthesize', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ text }),
      });
      
      if (!response.ok) {
        throw new Error('TTS synthesis failed');
      }
      
      const { audio_base64, content_type } = await response.json();
      
      // Convert base64 to audio and play
      const audioData = atob(audio_base64);
      const audioArray = new Uint8Array(audioData.length);
      for (let i = 0; i < audioData.length; i++) {
        audioArray[i] = audioData.charCodeAt(i);
      }
      
      const audioBlob = new Blob([audioArray], { type: content_type });
      const audioUrl = URL.createObjectURL(audioBlob);
      const audio = new Audio(audioUrl);
      
      audio.onended = () => {
        setIsPlaying(false);
        URL.revokeObjectURL(audioUrl);
      };
      
      await audio.play();
    } catch (error) {
      console.error("TTS error:", error);
      // Fallback to browser TTS
      if ('speechSynthesis' in window) {
        const utterance = new SpeechSynthesisUtterance(text);
        utterance.rate = 0.9;
        utterance.onstart = () => setIsPlaying(true);
        utterance.onend = () => setIsPlaying(false);
        window.speechSynthesis.speak(utterance);
      }
    }
  };

  // Send text message
  const handleTextSend = async () => {
    if (!input.trim() || isProcessing) return;
    
    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: "user",
      content: input,
      timestamp: new Date(),
      type: "text",
    };
    
    setMessages(prev => [...prev, userMessage]);
    setInput("");
    
    await sendMessage(input, "text");
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleTextSend();
    }
  };

  return (
    <div className="flex flex-col h-[80vh] max-w-6xl mx-auto p-4">
      <Card className="flex-1 flex flex-col bg-background/95 backdrop-blur border-border/50 shadow-lg">
        {/* Header */}
        <div className="p-4 border-b border-border/50">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <Bot className="h-5 w-5 text-primary" />
            AI Voice Assistant
          </h2>
          <p className="text-sm text-muted-foreground mt-1">
            {currentStatus} | Session: {sessionId ? sessionId.slice(0, 8) + "..." : "Not started"}
          </p>
        </div>

        <div className="flex-1 flex">
          {/* Left side - Voice Controls */}
          <div className="w-1/3 p-6 border-r border-border/50 flex flex-col items-center justify-center">
            <div className="relative">
              {/* Pulsing Ring Animation */}
              {isRecording && (
                <div className="absolute inset-0 rounded-full border-2 border-primary/30 animate-pulse-ring pointer-events-none scale-150" />
              )}
              
              {/* Voice Visualizer */}
              <div className="mb-8">
                <VoiceVisualizer isActive={isRecording || isPlaying} />
              </div>
              
              {/* Main Voice Button */}
              <Button
                onClick={toggleRecording}
                size="lg"
                disabled={isProcessing}
                className={cn(
                  "h-32 w-32 rounded-full transition-all duration-300 transform hover:scale-105",
                  isRecording
                    ? "bg-red-500 hover:bg-red-600 text-white shadow-lg animate-pulse"
                    : "bg-gradient-primary text-primary-foreground shadow-md hover:shadow-lg"
                )}
              >
                {isRecording ? (
                  <MicOff className="h-12 w-12" />
                ) : (
                  <Mic className="h-12 w-12" />
                )}
              </Button>
              
              <p className="text-center mt-4 text-sm text-muted-foreground">
                {isRecording ? "Click to stop recording" : "Click to start speaking"}
              </p>
            </div>
            
            {/* Voice Hints */}
            <div className="mt-8 text-center">
              <p className="text-xs text-muted-foreground mb-2">Try saying:</p>
              <div className="space-y-1">
                <div className="px-3 py-1 bg-muted rounded-full text-xs">
                  "What's the weather today?"
                </div>
                <div className="px-3 py-1 bg-muted rounded-full text-xs">
                  "Tell me a joke"
                </div>
                <div className="px-3 py-1 bg-muted rounded-full text-xs">
                  "Help me with coding"
                </div>
              </div>
            </div>
          </div>

          {/* Right side - Chat Messages */}
          <div className="flex-1 flex flex-col">
            {/* Messages Area */}
            <ScrollArea className="flex-1 p-4" ref={scrollAreaRef}>
              {messages.length === 0 ? (
                <div className="text-center text-muted-foreground py-8">
                  <Bot className="h-12 w-12 mx-auto mb-4 text-muted-foreground/50" />
                  <p>Start a conversation by speaking or typing</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {messages.map((message) => (
                    <div
                      key={message.id}
                      className={`flex gap-3 ${
                        message.role === "user" ? "justify-end" : "justify-start"
                      }`}
                    >
                      {message.role === "assistant" && (
                        <div className="flex-shrink-0">
                          <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                            <Bot className="h-5 w-5 text-primary" />
                          </div>
                        </div>
                      )}
                      <div
                        className={`max-w-[70%] rounded-lg px-4 py-2 ${
                          message.role === "user"
                            ? "bg-primary text-primary-foreground"
                            : "bg-muted"
                        }`}
                      >
                        <div className="flex items-center gap-2 mb-1">
                          {message.type === "voice" && (
                            <Volume2 className="h-3 w-3" />
                          )}
                          <p className="text-xs opacity-70">
                            {message.timestamp.toLocaleTimeString()}
                          </p>
                        </div>
                        <p className="text-sm">{message.content}</p>
                      </div>
                      {message.role === "user" && (
                        <div className="flex-shrink-0">
                          <div className="h-8 w-8 rounded-full bg-primary flex items-center justify-center">
                            <User className="h-5 w-5 text-primary-foreground" />
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                  {isProcessing && (
                    <div className="flex gap-3 justify-start">
                      <div className="flex-shrink-0">
                        <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                          <Bot className="h-5 w-5 text-primary animate-pulse" />
                        </div>
                      </div>
                      <div className="bg-muted rounded-lg px-4 py-2">
                        <div className="flex gap-1">
                          <span className="h-2 w-2 bg-primary/60 rounded-full animate-bounce" style={{ animationDelay: "0ms" }}></span>
                          <span className="h-2 w-2 bg-primary/60 rounded-full animate-bounce" style={{ animationDelay: "150ms" }}></span>
                          <span className="h-2 w-2 bg-primary/60 rounded-full animate-bounce" style={{ animationDelay: "300ms" }}></span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </ScrollArea>

            {/* Text Input Area */}
            <div className="p-4 border-t border-border/50">
              <div className="flex gap-2">
                <Input
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyPress={handleKeyPress}
                  placeholder="Type a message or use voice..."
                  disabled={isProcessing}
                  className="flex-1"
                />
                <Button
                  onClick={handleTextSend}
                  disabled={!input.trim() || isProcessing}
                  className="gap-2"
                >
                  <Send className="h-4 w-4" />
                  Send
                </Button>
              </div>
              <p className="text-xs text-muted-foreground mt-2">
                Press Enter to send, or click the microphone for voice input
              </p>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}