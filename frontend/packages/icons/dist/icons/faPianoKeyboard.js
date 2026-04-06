var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPianoKeyboard.js
var require_faPianoKeyboard = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPianoKeyboard.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "piano-keyboard";
    var width = 576;
    var height = 512;
    var aliases = [127929];
    var unicode = "f8d5";
    var svgPathData = "M64 64C28.7 64 0 92.7 0 128L0 384c0 35.3 28.7 64 64 64l448 0c35.3 0 64-28.7 64-64l0-256c0-35.3-28.7-64-64-64L64 64zM512 224l0 160-80 0 0-68.3c9.6-5.5 16-15.9 16-27.7l0-64 64 0zM400 384l-96 0 0-68.3c9.6-5.5 16-15.9 16-27.7l0-64 64 0 0 64c0 11.8 6.4 22.2 16 27.7l0 68.3zm-128 0l-96 0 0-68.3c9.6-5.5 16-15.9 16-27.7l0-64 64 0 0 64c0 11.8 6.4 22.2 16 27.7l0 68.3zm-128 0l-80 0 0-160 64 0 0 64c0 11.8 6.4 22.2 16 27.7l0 68.3z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faPianoKeyboard = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faPianoKeyboard.js
var import_faPianoKeyboard = __toESM(require_faPianoKeyboard());
var export_faPianoKeyboard = import_faPianoKeyboard.definition;
export {
  export_faPianoKeyboard as faPianoKeyboard
};
