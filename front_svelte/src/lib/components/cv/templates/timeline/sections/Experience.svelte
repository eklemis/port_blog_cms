<script lang="ts">
	import AppIcon from '$lib/icons/appui/AppIcon.svelte';
	let {
		designFont,
		displaySetting,
		experience: exp,
		secIndex,
		subIndex,
		sectionCreatedHandler
	} = $props();
	import { onMount } from 'svelte';

	let container: HTMLElement;
	onMount(() => {
		let scrollHeight = container.scrollHeight;
		console.log('Experience ' + exp.id + ' created!');
		console.log('scrollHeight', scrollHeight);
		sectionCreatedHandler({
			type: 'Experience->' + exp.title,
			secIndex: secIndex,
			subIndex: subIndex,
			scrollHeight
		});
	});

	function createTask(exp_idx: number, at_idx: number, withText: string) {
		const countItems = exp.bulletItems.length;
		if (at_idx === countItems - 1) {
			exp.bulletItems = [...exp.bulletItems, withText];
		} else {
			let frontItems = exp.bulletItems.slice(0, at_idx + 1);
			let backItems = exp.bulletItems.slice(at_idx + 1);
			exp.bulletItems = [...frontItems, withText, ...backItems];
		}
	}
	// Fungsi untuk menghapus task
	function deleteTask(exp_idx: number, index: number) {
		if (exp.bulletItems.length <= 1) return; // Pastikan minimal satu task
		exp.bulletItems = exp.bulletItems.filter((_: number, i: number) => i !== index);
	}
	function getCursorPosition(event: KeyboardEvent): {
		row: number;
		column: number;
		currentText: string;
	} {
		const target = event.target as HTMLElement;
		const row = parseInt(target.getAttribute('data-index') || '0', 10);
		const selection = window.getSelection();
		if (!selection || selection.rangeCount === 0) {
			return { row, column: 0, currentText: target.innerText };
		}
		const range = selection.getRangeAt(0);
		const preCaretRange = range.cloneRange();
		preCaretRange.selectNodeContents(target);
		preCaretRange.setEnd(range.endContainer, range.endOffset);
		const column = preCaretRange.toString().length;
		const currentText = target.innerText;
		return { row, column, currentText };
	}
	// Fungsi untuk menentukan apakah kursor di baris pertama atau terakhir
	function isCursorAtLineBoundary(event: KeyboardEvent): {
		isFirstLine: boolean;
		isLastLine: boolean;
	} {
		const target = event.target as HTMLElement;
		const selection = window.getSelection();
		if (!selection || selection.rangeCount === 0) {
			return { isFirstLine: true, isLastLine: true }; // Default jika tidak ada seleksi
		}
		const range = selection.getRangeAt(0);
		const text = target.innerText;
		const lines = text.split('\n').filter((line) => line.trim() !== '');
		if (lines.length === 0) {
			return { isFirstLine: true, isLastLine: true }; // Task kosong
		}

		// Hitung posisi kursor
		const preCaretRange = range.cloneRange();
		preCaretRange.selectNodeContents(target);
		preCaretRange.setEnd(range.endContainer, range.endOffset);
		const cursorText = preCaretRange.toString();
		const cursorLines = cursorText.split('\n').filter((line) => line.trim() !== '');

		// Tentukan baris kursor
		const isFirstLine = cursorLines.length <= 1 && cursorText.length <= lines[0].length;
		const isLastLine = cursorText.length >= text.length || cursorLines.length === lines.length;

		return { isFirstLine, isLastLine };
	}
	// Handler untuk keydown pada task
	function handleTaskKeydown(event: KeyboardEvent, exp_idx: number, task_idx: number) {
		if (!['Enter', 'Backspace', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
			return;
		}
		if (event.key === 'Enter') {
			event.preventDefault();
			const { column, currentText } = getCursorPosition(event);
			// Pisahkan teks pada posisi kursor
			const textBefore = currentText.slice(0, column);
			exp.bulletItems[task_idx] = textBefore;
			const textAfter = currentText.slice(column);
			createTask(exp_idx, task_idx, textAfter ?? '');
			// Pindahkan fokus ke task baru
			setTimeout(() => {
				const nextTask = document.querySelector(
					`li.task[data-id="task-${exp_idx}-${task_idx + 1}"]`
				) as HTMLElement;
				if (nextTask) {
					nextTask.focus();
					// Posisikan kursor di awal task baru
					const range = document.createRange();
					const sel = window.getSelection();
					range.setStart(nextTask.firstChild || nextTask, 0);
					range.collapse(true);
					sel?.removeAllRanges();
					sel?.addRange(range);
				}
			}, 0);
		} else if (event.key === 'Backspace' && !exp.bulletItems[task_idx].trim()) {
			event.preventDefault();
			deleteTask(exp_idx, task_idx);
			setTimeout(() => {
				const prevTask = document.querySelector(
					`li.task[data-id="task-${exp_idx}-${task_idx - 1}"]`
				) as HTMLElement;
				if (prevTask && task_idx > 0) {
					prevTask.focus();
					// Posisikan kursor di akhir teks task sebelumnya
					const textLength = prevTask.innerText.length;
					const range = document.createRange();
					const sel = window.getSelection();
					const textNode = prevTask.firstChild || prevTask;
					range.setStart(textNode, textLength);
					range.collapse(true);
					sel?.removeAllRanges();
					sel?.addRange(range);
				}
			}, 0);
		} else if (event.key === 'ArrowUp' && task_idx > 0) {
			const { isFirstLine } = isCursorAtLineBoundary(event);
			if (isFirstLine) {
				event.preventDefault();
				const prevTask = document.querySelector(
					`li.task[data-id="task-${exp_idx}-${task_idx - 1}"]`
				) as HTMLElement;
				if (prevTask) {
					prevTask.focus();
					// Posisikan kursor di akhir teks task sebelumnya
					const textLength = prevTask.innerText.length;
					const range = document.createRange();
					const sel = window.getSelection();
					const textNode = prevTask.firstChild || prevTask;
					range.setStart(textNode, textLength);
					range.collapse(true);
					sel?.removeAllRanges();
					sel?.addRange(range);
				}
			}
		} else if (event.key === 'ArrowDown' && task_idx < exp.bulletItems.length - 1) {
			const { isLastLine } = isCursorAtLineBoundary(event);
			if (isLastLine) {
				event.preventDefault();
				const nextTask = document.querySelector(
					`li.task[data-id="task-${exp_idx}-${task_idx + 1}"]`
				) as HTMLElement;
				if (nextTask) {
					nextTask.focus();
					// Posisikan kursor di awal teks task berikutnya
					const range = document.createRange();
					const sel = window.getSelection();
					range.setStart(nextTask.firstChild || nextTask, 0);
					range.collapse(true);
					sel?.removeAllRanges();
					sel?.addRange(range);
				}
			}
		}
	}
</script>

<section
	class="-ml-3 flex w-full flex-col gap-y-2 rounded-sm border border-transparent p-2 hover:border-blue-500"
	bind:this={container}
>
	<div class="flex flex-col gap-y-1">
		<h3
			contenteditable="true"
			class={`text-${designFont.primaryColor} text-lg`}
			bind:innerText={exp.title}
		>
			Section Title
		</h3>
		<h4 class={`text-${designFont.secondaryColor} text-base font-bold`}>{exp.companyName}</h4>
		{#if displaySetting.showCompanyDescription}
			<p>{exp.companyDescription}</p>
		{/if}
	</div>
	<ul class="flex max-w-2xl flex-wrap gap-x-4 gap-y-2">
		<li class="flex items-center gap-x-1.5">
			<span><AppIcon name="calendar" size="small" color="text-gray-700" /></span>{exp.period}
		</li>

		<li class="flex items-center gap-x-1.5">
			<span><AppIcon name="location" size="small" color="text-gray-700" /></span>{exp.location}
		</li>
	</ul>
	<ul class="ml-4 list-disc flex-col">
		{#each exp.bulletItems as task, task_idx ('task-' + exp.id + '-' + task_idx)}
			<li
				contenteditable="true"
				class="task outline-0"
				data-id={'task-' + exp.id + '-' + task_idx}
				onkeydown={(ev) => handleTaskKeydown(ev, exp.id, task_idx)}
				bind:innerText={exp.bulletItems[task_idx]}
			>
				{task}
			</li>
		{/each}
	</ul>
</section>
