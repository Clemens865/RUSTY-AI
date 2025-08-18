/**
 * Voice Integration Hooks for Personal AI Assistant
 * 
 * Provides React hooks for voice recording, transcription, and text-to-speech
 * functionality with seamless integration to the backend voice pipeline.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { API_CONFIG, API_ENDPOINTS, buildApiUrl, getAuthHeaders } from '../config/api-config';

// Voice-related types
export interface VoiceRecordingOptions {
  /** Maximum recording duration in milliseconds */
  maxDuration?: number;
  /** Audio sample rate */
  sampleRate?: number;
  /** Number of audio channels */
  channels?: number;
  /** Audio format for recording */
  format?: 'webm' | 'wav' | 'mp3';
  /** Enable voice activity detection */
  enableVAD?: boolean;
  /** VAD sensitivity (0-1) */
  vadSensitivity?: number;
}

export interface VoiceRecordingState {
  /** Whether currently recording */
  isRecording: boolean;
  /** Whether recording is paused */
  isPaused: boolean;
  /** Current recording duration in seconds */
  duration: number;
  /** Audio level (0-1) for visualization */
  audioLevel: number;
  /** Any recording error */
  error: string | null;
}

export interface TranscriptionOptions {
  /** Target language for transcription */
  language?: string;
  /** Enable real-time transcription */
  realtime?: boolean;
  /** Confidence threshold for accepting transcription */
  confidenceThreshold?: number;
}

export interface TranscriptionResult {
  /** Transcribed text */
  text: string;
  /** Confidence score (0-1) */
  confidence: number;
  /** Language detected */
  language: string;
  /** Duration of audio in milliseconds */
  duration: number;
}

export interface TTSOptions {
  /** Voice ID to use */
  voice?: string;
  /** Speech speed (0.25-4.0) */
  speed?: number;
  /** Audio format for output */
  format?: 'mp3' | 'wav' | 'ogg';
  /** Volume level (0-1) */
  volume?: number;
}

export interface TTSState {
  /** Whether currently synthesizing */
  isSynthesizing: boolean;
  /** Whether currently playing */
  isPlaying: boolean;
  /** Current playback position */
  position: number;
  /** Total duration */
  duration: number;
  /** Any synthesis error */
  error: string | null;
}

export interface AvailableVoice {
  id: string;
  name: string;
  language: string;
  gender: 'male' | 'female' | 'neutral';
  description: string;
}

/**
 * Hook for audio recording functionality
 */
export function useVoiceRecording(options: VoiceRecordingOptions = {}) {
  const {
    maxDuration = 300000, // 5 minutes default
    sampleRate = 44100,
    channels = 1,
    format = 'webm',
    enableVAD = true,
    vadSensitivity = 0.5,
  } = options;

  const [state, setState] = useState<VoiceRecordingState>({
    isRecording: false,
    isPaused: false,
    duration: 0,
    audioLevel: 0,
    error: null,
  });

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);
  const durationTimerRef = useRef<number | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  // Initialize audio context for level monitoring
  const initializeAudioContext = useCallback(async (stream: MediaStream) => {
    try {
      const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      const analyser = audioContext.createAnalyser();
      const source = audioContext.createMediaStreamSource(stream);
      
      analyser.fftSize = 256;
      source.connect(analyser);
      
      audioContextRef.current = audioContext;
      analyserRef.current = analyser;

      // Start monitoring audio levels
      const monitorAudioLevel = () => {
        if (!analyserRef.current) return;

        const dataArray = new Uint8Array(analyserRef.current.frequencyBinCount);
        analyserRef.current.getByteFrequencyData(dataArray);
        
        const average = dataArray.reduce((sum, value) => sum + value, 0) / dataArray.length;
        const level = average / 255;
        
        setState(prev => ({ ...prev, audioLevel: level }));
        
        animationFrameRef.current = requestAnimationFrame(monitorAudioLevel);
      };
      
      monitorAudioLevel();
    } catch (error) {
      console.warn('Failed to initialize audio context:', error);
    }
  }, []);

  // Start recording
  const startRecording = useCallback(async () => {
    try {
      setState(prev => ({ ...prev, error: null }));

      // Request microphone access
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          sampleRate,
          channelCount: channels,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      streamRef.current = stream;
      audioChunksRef.current = [];

      // Initialize audio level monitoring
      await initializeAudioContext(stream);

      // Set up MediaRecorder
      const mimeType = `audio/${format}`;
      const mediaRecorder = new MediaRecorder(stream, {
        mimeType: MediaRecorder.isTypeSupported(mimeType) ? mimeType : 'audio/webm',
      });

      mediaRecorderRef.current = mediaRecorder;

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
        }
      };

      mediaRecorder.onstart = () => {
        setState(prev => ({ ...prev, isRecording: true, duration: 0 }));
        
        // Start duration timer
        durationTimerRef.current = window.setInterval(() => {
          setState(prev => ({ ...prev, duration: prev.duration + 1 }));
        }, 1000);
      };

      mediaRecorder.onstop = () => {
        setState(prev => ({ ...prev, isRecording: false, isPaused: false }));
        
        // Clear timers
        if (durationTimerRef.current) {
          clearInterval(durationTimerRef.current);
          durationTimerRef.current = null;
        }
        
        if (animationFrameRef.current) {
          cancelAnimationFrame(animationFrameRef.current);
          animationFrameRef.current = null;
        }
      };

      // Start recording
      mediaRecorder.start(1000); // Collect data every second

      // Auto-stop after max duration
      setTimeout(() => {
        if (mediaRecorder.state === 'recording') {
          stopRecording();
        }
      }, maxDuration);

    } catch (error) {
      setState(prev => ({ 
        ...prev, 
        error: error instanceof Error ? error.message : 'Failed to start recording' 
      }));
    }
  }, [sampleRate, channels, format, maxDuration, initializeAudioContext]);

  // Stop recording
  const stopRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop();
    }
    
    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop());
      streamRef.current = null;
    }
    
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }
  }, []);

  // Pause recording
  const pauseRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'recording') {
      mediaRecorderRef.current.pause();
      setState(prev => ({ ...prev, isPaused: true }));
    }
  }, []);

  // Resume recording
  const resumeRecording = useCallback(() => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'paused') {
      mediaRecorderRef.current.resume();
      setState(prev => ({ ...prev, isPaused: false }));
    }
  }, []);

  // Get recorded audio as blob
  const getRecordingBlob = useCallback((): Blob | null => {
    if (audioChunksRef.current.length === 0) return null;
    
    const mimeType = `audio/${format}`;
    return new Blob(audioChunksRef.current, { 
      type: MediaRecorder.isTypeSupported(mimeType) ? mimeType : 'audio/webm' 
    });
  }, [format]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopRecording();
    };
  }, [stopRecording]);

  return {
    ...state,
    startRecording,
    stopRecording,
    pauseRecording,
    resumeRecording,
    getRecordingBlob,
  };
}

/**
 * Hook for speech-to-text transcription
 */
export function useTranscription(options: TranscriptionOptions = {}) {
  const {
    language = 'en',
    realtime = false,
    confidenceThreshold = 0.7,
  } = options;

  const [isTranscribing, setIsTranscribing] = useState(false);
  const [result, setResult] = useState<TranscriptionResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const transcribeAudio = useCallback(async (audioBlob: Blob): Promise<TranscriptionResult | null> => {
    setIsTranscribing(true);
    setError(null);

    try {
      const formData = new FormData();
      formData.append('audio', audioBlob);
      formData.append('language', language);

      const response = await fetch(buildApiUrl(API_ENDPOINTS.voice.transcribe), {
        method: 'POST',
        headers: {
          ...getAuthHeaders(),
        },
        body: formData,
      });

      if (!response.ok) {
        throw new Error(`Transcription failed: ${response.statusText}`);
      }

      const data = await response.json();
      
      if (!data.success) {
        throw new Error(data.error?.message || 'Transcription failed');
      }

      const transcriptionResult: TranscriptionResult = {
        text: data.data.text,
        confidence: data.data.confidence,
        language: data.data.language,
        duration: data.data.duration_ms,
      };

      // Only return result if confidence meets threshold
      if (transcriptionResult.confidence >= confidenceThreshold) {
        setResult(transcriptionResult);
        return transcriptionResult;
      } else {
        setError(`Low confidence transcription: ${transcriptionResult.confidence}`);
        return null;
      }

    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Transcription failed';
      setError(errorMessage);
      return null;
    } finally {
      setIsTranscribing(false);
    }
  }, [language, confidenceThreshold]);

  return {
    isTranscribing,
    result,
    error,
    transcribeAudio,
  };
}

/**
 * Hook for text-to-speech synthesis
 */
export function useTextToSpeech(options: TTSOptions = {}) {
  const {
    voice = API_CONFIG.voice.defaultVoice,
    speed = API_CONFIG.voice.defaultSpeed,
    format = 'mp3',
    volume = 1.0,
  } = options;

  const [state, setState] = useState<TTSState>({
    isSynthesizing: false,
    isPlaying: false,
    position: 0,
    duration: 0,
    error: null,
  });

  const audioRef = useRef<HTMLAudioElement | null>(null);

  const synthesizeText = useCallback(async (text: string): Promise<boolean> => {
    setState(prev => ({ ...prev, isSynthesizing: true, error: null }));

    try {
      const response = await fetch(buildApiUrl(API_ENDPOINTS.voice.synthesize), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...getAuthHeaders(),
        },
        body: JSON.stringify({
          text,
          voice,
          speed,
          format,
        }),
      });

      if (!response.ok) {
        throw new Error(`TTS failed: ${response.statusText}`);
      }

      const audioBlob = await response.blob();
      const audioUrl = URL.createObjectURL(audioBlob);
      
      // Create audio element for playback
      const audio = new Audio(audioUrl);
      audioRef.current = audio;

      audio.onloadedmetadata = () => {
        setState(prev => ({ ...prev, duration: audio.duration }));
      };

      audio.ontimeupdate = () => {
        setState(prev => ({ ...prev, position: audio.currentTime }));
      };

      audio.onplay = () => {
        setState(prev => ({ ...prev, isPlaying: true }));
      };

      audio.onpause = () => {
        setState(prev => ({ ...prev, isPlaying: false }));
      };

      audio.onended = () => {
        setState(prev => ({ ...prev, isPlaying: false, position: 0 }));
        URL.revokeObjectURL(audioUrl);
      };

      audio.volume = volume;

      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'TTS synthesis failed';
      setState(prev => ({ ...prev, error: errorMessage }));
      return false;
    } finally {
      setState(prev => ({ ...prev, isSynthesizing: false }));
    }
  }, [voice, speed, format, volume]);

  const play = useCallback(() => {
    audioRef.current?.play();
  }, []);

  const pause = useCallback(() => {
    audioRef.current?.pause();
  }, []);

  const stop = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
    }
  }, []);

  const setPosition = useCallback((position: number) => {
    if (audioRef.current) {
      audioRef.current.currentTime = position;
    }
  }, []);

  return {
    ...state,
    synthesizeText,
    play,
    pause,
    stop,
    setPosition,
  };
}

/**
 * Hook for managing available voices
 */
export function useVoices() {
  const [voices, setVoices] = useState<AvailableVoice[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadVoices = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(buildApiUrl(API_ENDPOINTS.voice.voices), {
        headers: getAuthHeaders(),
      });

      if (!response.ok) {
        throw new Error(`Failed to load voices: ${response.statusText}`);
      }

      const data = await response.json();
      
      if (!data.success) {
        throw new Error(data.error?.message || 'Failed to load voices');
      }

      setVoices(data.data.voices);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load voices';
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (API_CONFIG.features.voiceEnabled) {
      loadVoices();
    }
  }, [loadVoices]);

  return {
    voices,
    isLoading,
    error,
    loadVoices,
  };
}

/**
 * Combined hook for complete voice interaction workflow
 */
export function useVoiceInteraction(options: {
  recording?: VoiceRecordingOptions;
  transcription?: TranscriptionOptions;
  tts?: TTSOptions;
} = {}) {
  const recording = useVoiceRecording(options.recording);
  const transcription = useTranscription(options.transcription);
  const tts = useTextToSpeech(options.tts);
  const voices = useVoices();

  const [isProcessing, setIsProcessing] = useState(false);

  const recordAndTranscribe = useCallback(async (): Promise<string | null> => {
    setIsProcessing(true);
    
    try {
      // Start recording
      await recording.startRecording();
      
      return new Promise((resolve) => {
        // Auto-stop on silence detection or manual stop
        const checkForStop = () => {
          if (!recording.isRecording) {
            const audioBlob = recording.getRecordingBlob();
            if (audioBlob) {
              transcription.transcribeAudio(audioBlob).then((result) => {
                resolve(result?.text || null);
                setIsProcessing(false);
              });
            } else {
              resolve(null);
              setIsProcessing(false);
            }
          } else {
            setTimeout(checkForStop, 100);
          }
        };
        
        checkForStop();
      });
    } catch (error) {
      setIsProcessing(false);
      return null;
    }
  }, [recording, transcription]);

  const speakText = useCallback(async (text: string): Promise<boolean> => {
    const success = await tts.synthesizeText(text);
    if (success) {
      tts.play();
    }
    return success;
  }, [tts]);

  return {
    recording,
    transcription,
    tts,
    voices,
    isProcessing,
    recordAndTranscribe,
    speakText,
  };
}

export default useVoiceInteraction;