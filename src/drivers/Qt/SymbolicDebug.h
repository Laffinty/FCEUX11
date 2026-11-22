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
// SymbolicDebug.h
//
#ifndef __SYMBOLIC_DEBUG_H__
#define __SYMBOLIC_DEBUG_H__

#include <string>

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QGroupBox>
#include <QPushButton>
#include <QFont>
#include <QLabel>
#include <QTimer>
#include <QFont>
#include <QFrame>
#include <QGroupBox>
#include <QLineEdit>
#include <QPlainTextEdit>

#include "../../debugsymboltable.h"


class SymbolEditWindow : public QDialog
{
   Q_OBJECT

	public:
		SymbolEditWindow(QWidget *parent = 0);
		~SymbolEditWindow(void);

		void setAddr( int addrIn );
		void setBank( int bankIn );
		void setSym( debugSymbol_t *symIn );

		void retranslateUi(void);
	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

		void updateArraySensitivity(void);
		void setSymNameWithArray(int idx = 0);
		void determineArrayStart(void);

		QFont      font;
		QLineEdit *filepath;
		QLineEdit *addrEntry;
		QLineEdit *nameEntry;
		QPlainTextEdit *commentEntry;
		QLineEdit      *arraySize;
		QLineEdit      *arrayInit;
		QCheckBox      *isArrayBox;
		QCheckBox      *arrayNameOverWrite;
		QCheckBox      *arrayCommentOverWrite;
		QCheckBox      *commentHeadOnly;
		QCheckBox      *deleteBox;
		QLabel         *arraySizeLbl[2];
		QLabel         *arrayInitLbl;

		QPushButton *okButton;
		QPushButton *cancelButton;

		debugSymbol_t *sym;

		int  addr;
		int  bank;
		int  charWidth;

	public slots:
		void closeWindow(void);
		int  exec(void);

	private slots:
		void isArrayStateChanged( int state );
		void arrayCommentHeadOnlyChanged( int state );

};


int generateNLFilenameForBank(int bank, std::string &NLfilename);
int generateNLFilenameForAddress(int address, std::string &NLfilename);

#define  ASM_DEBUG_SYMS       0x0001
#define  ASM_DEBUG_REGS       0x0002
#define  ASM_DEBUG_REPLACE    0x0004
#define  ASM_DEBUG_ADDR_02X   0x0008
#define  ASM_DEBUG_TRACES     0x0010

int DisassembleWithDebug(int addr, uint8_t *opcode, int flags, char *str, debugSymbol_t *symOut = NULL, debugSymbol_t *symOut2 = NULL );

#endif
