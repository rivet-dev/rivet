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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownLeftAndArrowUpRightToCenter.js
var require_faArrowDownLeftAndArrowUpRightToCenter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faArrowDownLeftAndArrowUpRightToCenter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-down-left-and-arrow-up-right-to-center";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e092";
    var svgPathData = "M502.6 54.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L336 130.7 336 80c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-50.7 0L502.6 54.6zM80 272c-17.7 0-32 14.3-32 32s14.3 32 32 32l50.7 0L9.4 457.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L176 381.3l0 50.7c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128c0-17.7-14.3-32-32-32L80 272z";
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
    exports.faArrowDownLeftAndArrowUpRightToCenter = exports.definition;
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

// src/_entries/faArrowDownLeftAndArrowUpRightToCenter.js
var import_faArrowDownLeftAndArrowUpRightToCenter = __toESM(require_faArrowDownLeftAndArrowUpRightToCenter());
var export_faArrowDownLeftAndArrowUpRightToCenter = import_faArrowDownLeftAndArrowUpRightToCenter.definition;
export {
  export_faArrowDownLeftAndArrowUpRightToCenter as faArrowDownLeftAndArrowUpRightToCenter
};
