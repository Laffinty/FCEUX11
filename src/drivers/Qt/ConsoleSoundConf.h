/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
// GameSoundConf.h
//

#ifndef __GameSndH__
#define __GameSndH__

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QLabel>
#include <QSlider>
#include <QFrame>
#include <QGroupBox>
#include <QProgressBar>
#include <QTimer>

class ConsoleSndConfDialog_t : public QDialog
{
	Q_OBJECT

public:
	ConsoleSndConfDialog_t(QWidget *parent = 0);
	~ConsoleSndConfDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	int  sndQuality;
	QCheckBox *enaChkbox;
	QCheckBox *muteChkbox;
	QCheckBox *enaLowPass;
	QCheckBox *swapDutyChkbox;
	QCheckBox *useGlobalFocus;
	QComboBox *qualitySelect;
	QComboBox *rateSelect;
	QSlider *bufSizeSlider;
	QLabel *bufSizeLabel;
	QLabel *volLbl;
	QLabel *triLbl;
	QLabel *sqr1Lbl;
	QLabel *sqr2Lbl;
	QLabel *nseLbl;
	QLabel *pcmLbl;
	QLabel *starveLbl;
	QSlider *sqr2Slider;
	QSlider *nseSlider;
	QSlider *pcmSlider;
	QProgressBar *bufUsage;
	QTimer       *updateTimer;
	QPushButton *resetBtn;
	QPushButton *closeBtn;
	QGroupBox *frame1;
	QGroupBox *frame2;
	QGroupBox *frame3;

	void setCheckBoxFromProperty(QCheckBox *cbx, const char *property);
	void setComboBoxFromProperty(QComboBox *cbx, const char *property);
	void setSliderFromProperty(QSlider *slider, QLabel *lbl, const char *property);
	void setSliderEnables(void);

private slots:
	void closeWindow(void);
	void resetCounters(void);
	void periodicUpdate(void);
	void bufSizeChanged(int value);
	void volumeChanged(int value);
	void triangleChanged(int value);
	void square1Changed(int value);
	void square2Changed(int value);
	void noiseChanged(int value);
	void pcmChanged(int value);
	void enaSoundStateChange(int value);
	void enaSpeakerMuteChange(int value);
	void enaSoundLowPassChange(int value);
	void swapDutyCallback(int value);
	void useGlobalFocusChanged(int value);
	void soundQualityChanged(int index);
	void soundRateChanged(int index);
};

#endif
