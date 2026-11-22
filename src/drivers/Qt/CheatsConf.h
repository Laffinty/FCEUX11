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
// GamePadConf.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QLabel>
#include <QFrame>
#include <QLineEdit>
#include <QGroupBox>
#include <QTreeView>
#include <QTreeWidget>
#include <QTextEdit>

#include "Qt/main.h"

class GuiCheatsDialog_t : public QDialog
{
	Q_OBJECT

public:
	GuiCheatsDialog_t(QWidget *parent = 0);
	~GuiCheatsDialog_t(void);

	void retranslateUi(void);

	int addSearchResult(uint32_t a, uint8_t last, uint8_t current);

	int activeCheatListCB(const char *name, uint32 a, uint8 v, int c, int s, int type, void *data);

	void showActiveCheatList(bool redraw);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QGroupBox *actCheatFrame;
	QGroupBox *cheatSearchFrame;
	QGroupBox *cheatResultFrame;
	QPushButton *addCheatBtn;
	QPushButton *delCheatBtn;
	QPushButton *modCheatBtn;
	QPushButton *importCheatFileBtn;
	QPushButton *exportCheatFileBtn;
	QPushButton *srchResetBtn;
	QPushButton *knownValBtn;
	QPushButton *eqValBtn;
	QPushButton *neValBtn;
	QPushButton *grValBtn;
	QPushButton *ltValBtn;
	QCheckBox *useNeVal;
	QCheckBox *useGrVal;
	QCheckBox *useLtVal;
	QCheckBox *enaCheats;
	QCheckBox *autoSave;
	QCheckBox *pauseBox;
	QTreeWidget *actvCheatList;
	QTreeWidget *srchResults;
	QLineEdit *cheatNameEntry;
	QLineEdit *cheatAddrEntry;
	QLineEdit *cheatValEntry;
	QLineEdit *cheatCmpEntry;
	QLineEdit *knownValEntry;
	QLineEdit *neValEntry;
	QLineEdit *grValEntry;
	QLineEdit *ltValEntry;
	QComboBox *typeEntry;
	QFont font;

	int fontCharWidth;
	int actvCheatIdx;
	bool pauseWhileActive;
	bool wasPausedByCheats;

private:
	void showCheatSearchResults(void);

public slots:
	void closeWindow(void);
private slots:
	void resetSearchCallback(void);
	void knownValueCallback(void);
	void equalValueCallback(void);
	void notEqualValueCallback(void);
	void lessThanValueCallback(void);
	void greaterThanValueCallback(void);
	void openCheatFile(void);
	void saveCheatFile(void);
	void addActvCheat(void);
	void deleteActvCheat(void);
	void updateCheatParameters(void);
	void autoLoadSaveCheats(int state);
	void globalEnableCheats(int state);
	void pauseWindowState(int state);
	void actvCheatItemClicked(QTreeWidgetItem *item, int column);
};

void openCheatDialog(QWidget *parent);

void updateCheatDialog(void);
