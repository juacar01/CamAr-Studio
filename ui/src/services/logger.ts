export interface LogEntry {
  id: string;
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR';
  module: string;
  message: string;
}

class ClientLogger {
  private logs: LogEntry[] = [];
  private listeners: ((logs: LogEntry[]) => void)[] = [];

  public log(level: 'INFO' | 'WARN' | 'ERROR', module: string, message: string) {
    const entry: LogEntry = {
      id: Math.random().toString(36).substring(2, 9),
      timestamp: new Date().toLocaleTimeString(),
      level,
      module,
      message,
    };
    this.logs.push(entry);
    if (this.logs.length > 250) {
      this.logs.shift();
    }
    this.notify();

    if (level === 'ERROR') {
      console.error(`[${entry.module}] ${entry.message}`);
    } else if (level === 'WARN') {
      console.warn(`[${entry.module}] ${entry.message}`);
    } else {
      console.log(`[${entry.module}] ${entry.message}`);
    }
  }

  public info(module: string, message: string) {
    this.log('INFO', module, message);
  }

  public warn(module: string, message: string) {
    this.log('WARN', module, message);
  }

  public error(module: string, message: string) {
    this.log('ERROR', module, message);
  }

  public getLogs(): LogEntry[] {
    return [...this.logs];
  }

  public clear() {
    this.logs = [];
    this.notify();
  }

  public subscribe(listener: (logs: LogEntry[]) => void) {
    this.listeners.push(listener);
    listener([...this.logs]);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    for (const listener of this.listeners) {
      listener([...this.logs]);
    }
  }
}

export const uiLogger = new ClientLogger();
