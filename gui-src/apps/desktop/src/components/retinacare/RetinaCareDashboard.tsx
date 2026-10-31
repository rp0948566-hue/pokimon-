import React, { useState } from 'react';
import { LiquidButton } from '@/components/ui/liquid-glass-button';

const FUNDUS_MODES = [
  {
    id: 'original',
    title: 'Original Image',
    subtitle: 'OD - Right Eye',
  },
  {
    id: 'lesion',
    title: 'AI Lesion Detection',
    subtitle: '3 findings detected',
  },
  {
    id: 'vessel',
    title: 'Vessel Analysis',
    subtitle: 'Vessel density • AV ratio',
  },
  {
    id: 'gradcam',
    title: 'Grad-CAM',
    subtitle: 'AI attention map',
  },
] as const;

export const RetinaCareDashboard: React.FC = () => {
  const [isSidebarOpen, setIsSidebarOpen] = useState<boolean>(true);
  const [selectedFundusMode, setSelectedFundusMode] = useState<string>('original');
  const [uploadedImage, setUploadedImage] = useState<string | null>(null);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (event) => {
        if (event.target?.result) {
          setUploadedImage(event.target.result as string);
        }
      };
      reader.readAsDataURL(file);
    }
  };

  return (
    <div className="retinacare-app-container">
      {/* Outer Shell Wrapper */}
      <div className="retinacare-window">
        {/* Animated Sidebar Toggle Button */}
        <button
          type="button"
          className={`sidebar-toggle-btn ${isSidebarOpen ? 'open' : 'closed'}`}
          title={isSidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
          aria-label={isSidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
          onClick={() => setIsSidebarOpen(!isSidebarOpen)}
        >
          <svg
            className="sidebar-icon"
            width="18"
            height="18"
            viewBox="0 0 20 20"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <rect className="sidebar-icon-box" x="2.5" y="3.5" width="15" height="13" rx="2.5" />
            <line className="sidebar-icon-line-full" x1="7.5" y1="3.5" x2="7.5" y2="16.5" />
            <line className="sidebar-icon-line-short" x1="7.5" y1="5.5" x2="7.5" y2="14.5" />
            <path
              className="sidebar-icon-arrow"
              d={isSidebarOpen ? "M12.5 7.5L9.5 10L12.5 12.5" : "M9.5 7.5L12.5 10L9.5 12.5"}
            />
          </svg>
        </button>

        {/* ================= LEFT SIDEBAR ================= */}
        <aside className={`retinacare-sidebar ${!isSidebarOpen ? 'collapsed' : ''}`} />

        {/* ================= MAIN CONTENT AREA ================= */}
        <main className={`retinacare-main ${!isSidebarOpen ? 'sidebar-collapsed' : ''}`}>
          {/* Top Header Bar */}
          <header className="main-header">
            <div className="header-actions">
              <LiquidButton
                variant="default"
                size="xl"
                className="liquid-btn-upload rounded-full px-6 py-3 text-[15px] font-semibold cursor-pointer"
                onClick={() => fileInputRef.current?.click()}
              >
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                  <polyline points="17 8 12 3 7 8" />
                  <line x1="12" y1="3" x2="12" y2="15" />
                </svg>
                <span>Upload Image</span>
              </LiquidButton>

              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                style={{ display: 'none' }}
                aria-label="Upload fundus image"
                onChange={handleFileChange}
              />
            </div>
          </header>

          {/* ================= HERO WORKSPACE ================= */}
          <div className="workspace-hero-row">
            {/* Center Expanded Soft-Black Display Box (#262727) */}
            <div className="workspace-center-box-container">
              <div
                className="workspace-center-box"
                onClick={() => !uploadedImage && fileInputRef.current?.click()}
                style={{ cursor: !uploadedImage ? 'pointer' : 'default' }}
              >
                {uploadedImage ? (
                  <div className="center-preview-image-wrapper active">
                    <img
                      src={uploadedImage}
                      alt={selectedFundusMode}
                      className="center-preview-image"
                    />
                  </div>
                ) : (
                  <div className="center-empty-state">
                    <svg viewBox="0 0 24 24" width="44" height="44" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" className="empty-upload-icon">
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                      <polyline points="17 8 12 3 7 8" />
                      <line x1="12" y1="3" x2="12" y2="15" />
                    </svg>
                    <span>Click or use Upload Image to view</span>
                  </div>
                )}
              </div>
            </div>

            {/* Right Card: Fundus Image & AI Analysis (2x2 Grid) */}
            <div className="card-fundus-analysis">
              <div className="card-title-header">
                <div className="card-title-left">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="card-header-icon">
                    <rect x="3" y="4" width="18" height="17" rx="3.5" />
                    <circle cx="8" cy="9.5" r="1.5" />
                    <path d="M14 8l1 1.5 1.5 1-1.5 1-1 1.5-1-1.5-1.5-1 1.5-1z" />
                  </svg>
                  <h3>Fundus Image & AI Analysis</h3>
                </div>
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" className="card-header-arrow">
                  <polyline points="9 18 15 12 9 6" />
                </svg>
              </div>

              <div className="fundus-grid">
                {FUNDUS_MODES.map((mode) => (
                  <div
                    key={mode.id}
                    className={`fundus-grid-item ${selectedFundusMode === mode.id ? 'selected' : ''}`}
                    onClick={() => {
                      if (uploadedImage) setSelectedFundusMode(mode.id);
                      else fileInputRef.current?.click();
                    }}
                  >
                    <div className="fundus-preview-thumb">
                      {uploadedImage ? (
                        <img src={uploadedImage} alt={mode.title} />
                      ) : (
                        <div className="thumb-empty-placeholder">
                          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                            <rect x="3" y="3" width="18" height="18" rx="3" />
                            <circle cx="8.5" cy="8.5" r="1.5" />
                            <polyline points="21 15 16 10 5 21" />
                          </svg>
                        </div>
                      )}
                    </div>
                    <div className="fundus-item-footer">
                      <div>
                        <div className="fundus-label">{mode.title}</div>
                        <div className="fundus-sublabel">{mode.subtitle}</div>
                      </div>
                      <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" className="fundus-item-arrow">
                        <polyline points="9 18 15 12 9 6" />
                      </svg>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
};
