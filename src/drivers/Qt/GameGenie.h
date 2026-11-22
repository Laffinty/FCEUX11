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
// GameGenie.h
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
#include <QGroupBox>
#include <QLineEdit>
#include <QTreeView>
#include <QTreeWidget>

#include "Qt/main.h"
#include "Qt/ConsoleUtilities.h"

class GameGenieDialog_t : public QDialog
{
   Q_OBJECT

	public:
		GameGenieDialog_t(QWidget *parent = 0);
		~GameGenieDialog_t(void);

		void retranslateUi(void);
	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

		fceuHexIntValidtor *addrValidator;
		fceuHexIntValidtor *cmpValidator;
		fceuHexIntValidtor *valValidator;

		QLineEdit   *addr;
		QLineEdit   *cmp;
		QLineEdit   *val;
		QLineEdit   *ggCode;
		QPushButton *addCheatBtn;
		QTreeWidget *tree;

	private:
		void ListGGAddresses(void);

   public slots:
      void closeWindow(void);
	private slots:
		void addCheatClicked(void);
		void addrChanged(const QString &);
		void cmpChanged(const QString &);
		void valChanged(const QString &);
		void ggChanged(const QString &);
		void romAddrDoubleClicked(QTreeWidgetItem *item, int column);

};

void EncodeGG(char *str, int a, int v, int c);
