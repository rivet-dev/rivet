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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDownLeft.js
var require_faSquareDownLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareDownLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-down-left";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e26b";
    var svgPathData = "M64 480c-35.3 0-64-28.7-64-64L0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480zM288 352c6.5 0 12.3-3.9 14.8-9.9s1.1-12.9-3.5-17.4l-40-40 71-71c3.6-3.6 5.7-8.5 5.7-13.7s-2-10-5.7-13.7l-36.7-36.7C290 146 285.1 144 280 144s-10 2-13.7 5.7l-71 71-40-40c-4.6-4.6-11.5-5.9-17.4-3.5s-9.9 8.3-9.9 14.8l0 144c0 8.8 7.2 16 16 16l144 0z";
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
    exports.faSquareDownLeft = exports.definition;
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

// src/_entries/faSquareDownLeft.js
var import_faSquareDownLeft = __toESM(require_faSquareDownLeft());
var export_faSquareDownLeft = import_faSquareDownLeft.definition;
export {
  export_faSquareDownLeft as faSquareDownLeft
};
