// TasEditorContext.h
//

#pragma once

#include <QObject>
#include <memory>

class TasEditorContext : public QObject
{
	Q_OBJECT

public:
	TasEditorContext(QObject *parent = nullptr)
		: QObject(parent)
		, selectionStart(-1)
		, selectionEnd(-1)
		, hasSelection(false)
	{
	}

	struct MovieData;

	int  selectionStart;
	int  selectionEnd;
	bool hasSelection;

	struct GreenzoneData;

signals:
	void frameChanged(int newFrame);
	void selectionChanged();
	void greenzoneUpdated();
	void bookmarkChanged();
	void branchChanged();
};
