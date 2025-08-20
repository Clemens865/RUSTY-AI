import { useState, useEffect } from "react";
import { Clock, MessageSquare, Trash2, Play, RefreshCw, ChevronRight } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";

interface ChatSession {
  id: string;
  created_at: string;
  updated_at: string;
  metadata?: any;
}

interface Message {
  id: string;
  role: string;
  content: string;
  created_at: string;
}

interface SessionDetails {
  session_id: string;
  messages: Message[];
  summary?: string;
  total: number;
}

export function History() {
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [showSessionDialog, setShowSessionDialog] = useState(false);
  const [loadingSession, setLoadingSession] = useState(false);

  // Fetch sessions from backend
  const fetchSessions = async () => {
    setIsLoading(true);
    try {
      const response = await fetch('/api/v1/conversation/sessions');
      if (response.ok) {
        const data = await response.json();
        setSessions(data.sessions || []);
      }
    } catch (error) {
      console.error("Failed to fetch sessions:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Fetch messages for a specific session
  const fetchSessionMessages = async (sessionId: string) => {
    setLoadingSession(true);
    try {
      const response = await fetch(`/api/v1/conversation/session/${sessionId}`);
      if (response.ok) {
        const data = await response.json();
        setSelectedSession(data);
        setShowSessionDialog(true);
      }
    } catch (error) {
      console.error("Failed to fetch session messages:", error);
    } finally {
      setLoadingSession(false);
    }
  };

  // Load sessions on mount
  useEffect(() => {
    fetchSessions();
  }, []);

  // Format date for display
  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(hours / 24);

    if (hours < 1) {
      return "Just now";
    } else if (hours < 24) {
      return `${hours} hour${hours > 1 ? 's' : ''} ago`;
    } else if (days < 7) {
      return `${days} day${days > 1 ? 's' : ''} ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  // Calculate session duration (time between first and last message in the session)
  const calculateDuration = (createdAt: string, updatedAt: string) => {
    const start = new Date(createdAt);
    const end = new Date(updatedAt);
    const diff = end.getTime() - start.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const seconds = Math.floor((diff % (1000 * 60)) / 1000);
    
    if (minutes === 0) {
      return `${seconds}s`;
    }
    return `${minutes}m ${seconds}s`;
  };

  return (
    <div className="p-6 max-w-4xl mx-auto animate-fade-in">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-foreground">Conversation History</h2>
        <div className="flex gap-2">
          <Button 
            variant="outline" 
            size="sm" 
            onClick={fetchSessions}
            disabled={isLoading}
            className="hover:shadow-md transition-all duration-300"
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
      </div>
      
      <div className="space-y-4">
        {isLoading && sessions.length === 0 ? (
          <div className="text-center py-12">
            <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-4" />
            <p className="text-muted-foreground">Loading conversations...</p>
          </div>
        ) : sessions.length > 0 ? (
          sessions.map((session, index) => (
            <Card 
              key={session.id} 
              className="bg-gradient-bg border-border/50 shadow-md hover:shadow-lg transition-all duration-300 animate-slide-up cursor-pointer"
              style={{ animationDelay: `${index * 0.1}s` }}
              onClick={() => fetchSessionMessages(session.id)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <CardTitle className="text-lg flex items-center gap-2">
                      <MessageSquare className="h-5 w-5 text-primary" />
                      Session {session.id.substring(0, 8)}
                    </CardTitle>
                    <CardDescription className="flex items-center gap-4 text-sm">
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {formatDate(session.created_at)}
                      </span>
                      <span>Duration: {calculateDuration(session.created_at, session.updated_at)}</span>
                    </CardDescription>
                  </div>
                  <Badge className="bg-success text-success-foreground">
                    completed
                  </Badge>
                </div>
              </CardHeader>
              
              <CardContent className="pt-0">
                <div className="flex items-center justify-between">
                  <p className="text-sm text-muted-foreground">
                    Click to view conversation details
                  </p>
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                </div>
              </CardContent>
            </Card>
          ))
        ) : (
          <div className="text-center py-12">
            <MessageSquare className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
            <h3 className="text-lg font-medium text-foreground mb-2">No conversation history yet</h3>
            <p className="text-muted-foreground">
              Start a conversation to see your history here
            </p>
          </div>
        )}
      </div>

      {/* Session Details Dialog */}
      <Dialog open={showSessionDialog} onOpenChange={setShowSessionDialog}>
        <DialogContent className="max-w-3xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>
              Conversation {selectedSession?.session_id.substring(0, 8)}
            </DialogTitle>
          </DialogHeader>
          
          {loadingSession ? (
            <div className="text-center py-8">
              <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-4" />
              <p className="text-muted-foreground">Loading conversation...</p>
            </div>
          ) : selectedSession ? (
            <div className="space-y-4">
              {selectedSession.summary && (
                <Card className="bg-muted/50">
                  <CardContent className="pt-4">
                    <h4 className="font-semibold mb-2">Summary</h4>
                    <p className="text-sm text-muted-foreground">{selectedSession.summary}</p>
                  </CardContent>
                </Card>
              )}
              
              <ScrollArea className="h-[400px] pr-4">
                <div className="space-y-4">
                  {selectedSession.messages.map((message, idx) => (
                    <div
                      key={message.id}
                      className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
                    >
                      <div
                        className={`max-w-[70%] rounded-lg p-3 ${
                          message.role === 'user'
                            ? 'bg-primary text-primary-foreground'
                            : 'bg-muted'
                        }`}
                      >
                        <p className="text-xs font-semibold mb-1">
                          {message.role === 'user' ? 'You' : 'Assistant'}
                        </p>
                        <p className="text-sm">{message.content}</p>
                        <p className="text-xs opacity-70 mt-1">
                          {new Date(message.created_at).toLocaleTimeString()}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
              
              <div className="text-center text-sm text-muted-foreground">
                {selectedSession.total} messages in this conversation
              </div>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}