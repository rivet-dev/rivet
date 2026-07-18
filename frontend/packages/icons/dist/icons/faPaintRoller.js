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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPaintRoller.js
var require_faPaintRoller = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPaintRoller.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paint-roller";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5aa";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L352 0c35.3 0 64 28.7 64 64l16 0c44.2 0 80 35.8 80 80l0 96c0 44.2-35.8 80-80 80l-160 0c-8.8 0-16 7.2-16 16l0 18.7c18.6 6.6 32 24.4 32 45.3l0 96c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48l0-96c0-20.9 13.4-38.7 32-45.3l0-18.7c0-44.2 35.8-80 80-80l160 0c8.8 0 16-7.2 16-16l0-96c0-8.8-7.2-16-16-16l-16 0c0 35.3-28.7 64-64 64L64 192c-35.3 0-64-28.7-64-64L0 64z";
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
    exports.faPaintRoller = exports.definition;
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

// src/_entries/faPaintRoller.js
var import_faPaintRoller = __toESM(require_faPaintRoller());
var export_faPaintRoller = import_faPaintRoller.definition;
export {
  export_faPaintRoller as faPaintRoller
};
