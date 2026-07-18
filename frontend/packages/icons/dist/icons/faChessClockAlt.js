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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChessClockFlip.js
var require_faChessClockFlip = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChessClockFlip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chess-clock-flip";
    var width = 640;
    var height = 512;
    var aliases = ["chess-clock-alt"];
    var unicode = "f43e";
    var svgPathData = "M384 56c0-13.3 10.7-24 24-24l112 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-32 0 0 48 88 0c35.3 0 64 28.7 64 64l0 224c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 192c0-35.3 28.7-64 64-64l32 0 0-16c0-8.8 7.2-16 16-16l128 0c8.8 0 16 7.2 16 16l0 16 184 0 0-48-32 0c-13.3 0-24-10.7-24-24zM288 304A112 112 0 1 0 64 304a112 112 0 1 0 224 0zm-52.7-59.3c6.2 6.2 6.2 16.4 0 22.6l-48 48c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6l48-48c6.2-6.2 16.4-6.2 22.6 0zM464 416a112 112 0 1 0 0-224 112 112 0 1 0 0 224zm16-176l0 64c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-64c0-8.8 7.2-16 16-16s16 7.2 16 16z";
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
    exports.faChessClockFlip = exports.definition;
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

// src/_entries/faChessClockAlt.js
var import_faChessClockFlip = __toESM(require_faChessClockFlip());
var export_faChessClockAlt = import_faChessClockFlip.definition;
export {
  export_faChessClockAlt as faChessClockAlt
};
