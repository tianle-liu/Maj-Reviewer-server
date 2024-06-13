from flask import Flask, request, render_template, send_file, jsonify, after_this_request
import os
import subprocess
import sys
import time
from werkzeug.utils import secure_filename

app = Flask(__name__)

UPLOAD_FOLDER = 'uploads'
MAX_FOLDER_SIZE_MB = 100  # Maximum folder size in MB
os.makedirs(UPLOAD_FOLDER, exist_ok=True)
app.config['UPLOAD_FOLDER'] = UPLOAD_FOLDER
app.config['MAX_CONTENT_LENGTH'] = 0.1 * 1024 * 1024  # 100 KB max file size


def get_folder_size(folder):
    total_size = 0
    for dirpath, dirnames, filenames in os.walk(folder):
        for f in filenames:
            fp = os.path.join(dirpath, f)
            total_size += os.path.getsize(fp)
    return total_size

def delete_oldest_files(folder, target_size_mb):
    files = sorted(
        (os.path.join(folder, f) for f in os.listdir(folder)),
        key=os.path.getctime
    )
    total_size = get_folder_size(folder)
    target_size = target_size_mb * 1024 * 1024
    while total_size > target_size and files:
        oldest_file = files.pop(0)
        try:
            os.remove(oldest_file)
            total_size = get_folder_size(folder)
        except Exception as e:
            app.logger.error(f"Error deleting file {oldest_file}: {e}")

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/upload', methods=['POST'])
def upload_file():
    if 'file' not in request.files:
        return jsonify({"error": "No file part"}), 400
    file = request.files['file']
    if file.filename == '':
        return jsonify({"error": "No selected file"}), 400
    if file:
        # Create a unique filename by appending a timestamp
        # original_filename = file.filename
        original_filename = secure_filename(file.filename) # secure_filename() is used to sanitize the filename
        filename, ext = os.path.splitext(original_filename)
        timestamp = int(time.time())
        unique_filename = f"{filename}_{timestamp}{ext}"
        filepath = os.path.join(UPLOAD_FOLDER, unique_filename)
        file.save(filepath)

        # Get the player ID from the form data
        player_id = request.form.get('player_id', 0)  # default to 0 if not provided

        # Check and delete old files if the folder size exceeds the limit
        delete_oldest_files(UPLOAD_FOLDER, MAX_FOLDER_SIZE_MB)

        # Run the mjai-reviewer command, which outputs the HTML to the same directory as the JSON file
        # subprocess.run(['/home/ubuntu/poronkusema/Reviewer-server/mjai-reviewer', '-e', 'mortal', '-i', filepath, '-a', str(player_id)])
        try:
            subprocess.run(['/home/ubuntu/poronkusema/Reviewer-server/mjai-reviewer', '-e', 'mortal', '-i', filepath, '-a', str(player_id)], check=True)
        except subprocess.CalledProcessError as e:
            app.logger.error(f"Error running mjai-reviewer: {e}")
            return jsonify({"error": "Error processing file"}), 500
        # Determine the output HTML file path
        output_filepath = filepath + '.html'

        return jsonify({"filepath": output_filepath})

@app.route('/result/<path:filepath>')
def result(filepath):
    # @after_this_request
    # def remove_file(response):
    #     try:
    #         # Remove the HTML file and the JSON file after the response has been sent
    #         os.remove(filepath)
    #         os.remove(filepath + '.html')
    #     except Exception as error:
    #         app.logger.error("Error removing file: %s", error)
    #     return response
    if not os.path.exists(filepath):
        return jsonify({"error": "File not found"}), 404

    return send_file(filepath)

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000, debug=True)
