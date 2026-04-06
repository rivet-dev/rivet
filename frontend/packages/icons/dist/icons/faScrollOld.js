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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScrollOld.js
var require_faScrollOld = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScrollOld.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scroll-old";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f70f";
    var svgPathData = "M48 32c26.5 0 48 21.5 48 48l0 80-48 0-16 0c-17.7 0-32-14.3-32-32L0 80C0 53.5 21.5 32 48 32zm80 48c0-18-6-34.6-16-48l272 0c51.4 0 93.4 40.5 95.9 91.3c-.4 3.5-2 6.9-4.6 9.4l-21.7 21.7c-3.1 3.1-3.1 8.2 0 11.3l21.7 21.7c3 3 4.7 7.1 4.7 11.3l0 1.4 0 16 0 1.4c0 4.2-1.7 8.3-4.7 11.3l-21.7 21.7c-3.1 3.1-3.1 8.2 0 11.3l21.7 21.7c3 3 4.7 7.1 4.7 11.3l0 1.4 0 24-165.3 0c-32.4 0-58.7 26.3-58.7 58.7l0 5.3c0 35.3-28.7 64-64 64s-64-28.7-64-64l0-120 0-1.4c0-4.2 1.7-8.3 4.7-11.3l21.7-21.7c3.1-3.1 3.1-8.2 0-11.3l-21.7-21.7c-3-3-4.7-7.1-4.7-11.3l0-1.4 0-104zM576 368c0 61.9-50.1 112-112 112l-96 0-176 0c53 0 96-43 96-96l0-5.3c0-14.7 11.9-26.7 26.7-26.7l29.3 0 1.4 0c4.2 0 8.3 1.7 11.3 4.7l21.7 21.7c3.1 3.1 8.2 3.1 11.3 0l21.7-21.7c3-3 7.1-4.7 11.3-4.7l1.4 0 136 0c8.8 0 16 7.2 16 16z";
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
    exports.faScrollOld = exports.definition;
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

// src/_entries/faScrollOld.js
var import_faScrollOld = __toESM(require_faScrollOld());
var export_faScrollOld = import_faScrollOld.definition;
export {
  export_faScrollOld as faScrollOld
};
