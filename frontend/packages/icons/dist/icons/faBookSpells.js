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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBookSparkles.js
var require_faBookSparkles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBookSparkles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "book-sparkles";
    var width = 448;
    var height = 512;
    var aliases = ["book-spells"];
    var unicode = "f6b8";
    var svgPathData = "M96 0C43 0 0 43 0 96L0 416c0 53 43 96 96 96l288 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-64c17.7 0 32-14.3 32-32l0-320c0-17.7-14.3-32-32-32L384 0 96 0zm0 384l256 0 0 64L96 448c-17.7 0-32-14.3-32-32s14.3-32 32-32zM264 200l16.6-38.8c2.8-6.5 11.9-6.5 14.7 0L312 200l38.8 16.6c6.5 2.8 6.5 11.9 0 14.7L312 248l-16.6 38.8c-2.8 6.5-11.9 6.5-14.7 0L264 248l-38.8-16.6c-6.5-2.8-6.5-11.9 0-14.7L264 200zM168.8 78.3c2.9-5.9 11.4-5.9 14.3 0L192 96l17.7 8.8c5.9 2.9 5.9 11.4 0 14.3L192 128l-8.8 17.7c-2.9 5.9-11.4 5.9-14.3 0L160 128l-17.7-8.8c-5.9-2.9-5.9-11.4 0-14.3L160 96l8.8-17.7z";
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
    exports.faBookSparkles = exports.definition;
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

// src/_entries/faBookSpells.js
var import_faBookSparkles = __toESM(require_faBookSparkles());
var export_faBookSpells = import_faBookSparkles.definition;
export {
  export_faBookSpells as faBookSpells
};
