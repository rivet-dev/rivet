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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowUpRightDots.js
var require_faArrowUpRightDots = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowUpRightDots.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-up-right-dots";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4b7";
    var svgPathData = "M96 32C78.3 32 64 46.3 64 64S78.3 96 96 96L114.7 96 9.4 201.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L160 141.3 160 160c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96c0-17.7-14.3-32-32-32L96 32zM403.8 70.1a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0zM279.7 194.2a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0zm162.2-38.1a38.1 38.1 0 1 0 0 76.2 38.1 38.1 0 1 0 0-76.2zM156.2 317.8a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0zm161.6-38.1a38.1 38.1 0 1 0 0 76.2 38.1 38.1 0 1 0 0-76.2zm86.1 38.1a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0zM70.1 403.8a38.1 38.1 0 1 0 0 76.2 38.1 38.1 0 1 0 0-76.2zm86.1 38.1a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0zm161.6-38.1a38.1 38.1 0 1 0 0 76.2 38.1 38.1 0 1 0 0-76.2zm86.1 38.1a38.1 38.1 0 1 0 76.2 0 38.1 38.1 0 1 0 -76.2 0z";
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
    exports.faArrowUpRightDots = exports.definition;
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

// src/_entries/faArrowUpRightDots.js
var import_faArrowUpRightDots = __toESM(require_faArrowUpRightDots());
var export_faArrowUpRightDots = import_faArrowUpRightDots.definition;
export {
  export_faArrowUpRightDots as faArrowUpRightDots
};
