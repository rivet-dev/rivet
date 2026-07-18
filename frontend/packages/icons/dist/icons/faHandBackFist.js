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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandBackFist.js
var require_faHandBackFist = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandBackFist.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-back-fist";
    var width = 384;
    var height = 512;
    var aliases = ["hand-rock"];
    var unicode = "f255";
    var svgPathData = "M7.4 253.6C2.6 245.9 0 237.1 0 228l0-36c0-26.5 21.5-48 48-48l16 0 0-80c0-26.5 21.5-48 48-48 17.3 0 32.4 9.1 40.9 22.8 4.3-22.1 23.8-38.8 47.1-38.8 23.4 0 42.9 16.8 47.1 38.9 7.3-4.4 15.8-6.9 24.9-6.9 22.1 0 40.8 15 46.3 35.4 5.5-2.2 11.4-3.4 17.7-3.4 26.5 0 48 21.5 48 48l0 96.9c0 9.9-2.3 19.7-6.8 28.6l-39.6 79.1c-10.8 21.7-33 35.4-57.2 35.4L96 352c-16.5 0-31.8-8.4-40.6-22.4l-48-76zM32 480l0-48c0-17.7 14.3-32 32-32l256 0c17.7 0 32 14.3 32 32l0 48c0 17.7-14.3 32-32 32L64 512c-17.7 0-32-14.3-32-32z";
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
    exports.faHandBackFist = exports.definition;
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

// src/_entries/faHandBackFist.js
var import_faHandBackFist = __toESM(require_faHandBackFist());
var export_faHandBackFist = import_faHandBackFist.definition;
export {
  export_faHandBackFist as faHandBackFist
};
