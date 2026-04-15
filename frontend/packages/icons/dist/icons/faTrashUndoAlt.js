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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrashCanUndo.js
var require_faTrashCanUndo = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrashCanUndo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trash-can-undo";
    var width = 448;
    var height = 512;
    var aliases = ["trash-can-arrow-turn-left", "trash-undo-alt"];
    var unicode = "f896";
    var svgPathData = "M135.2 17.7L128 32 32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-96 0-7.2-14.3C307.4 6.8 296.3 0 284.2 0L163.8 0c-12.1 0-23.2 6.8-28.6 17.7zM416 128L32 128l0 320c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-320zM217.8 199.9c8.9 9.9 8.1 25-1.8 33.9L182.5 264l57.5 0c48.6 0 88 39.4 88 88l0 40c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-40c0-22.1-17.9-40-40-40l-57.5 0 33.5 30.2c9.9 8.9 10.7 24 1.8 33.9s-24 10.7-33.9 1.8l-80-72c-5.1-4.6-7.9-11-7.9-17.8s2.9-13.3 7.9-17.8l80-72c9.9-8.9 25-8.1 33.9 1.8z";
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
    exports.faTrashCanUndo = exports.definition;
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

// src/_entries/faTrashUndoAlt.js
var import_faTrashCanUndo = __toESM(require_faTrashCanUndo());
var export_faTrashUndoAlt = import_faTrashCanUndo.definition;
export {
  export_faTrashUndoAlt as faTrashUndoAlt
};
