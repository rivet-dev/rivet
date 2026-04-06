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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBullseyePointer.js
var require_faBullseyePointer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBullseyePointer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bullseye-pointer";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f649";
    var svgPathData = "M256 448c106 0 192-86 192-192s-86-192-192-192S64 150 64 256c0 3.9 .1 7.8 .4 11.7L1.8 286.1C.6 276.2 0 266.2 0 256C0 114.6 114.6 0 256 0S512 114.6 512 256s-114.6 256-256 256c-10.2 0-20.2-.6-30.1-1.8l18.4-62.6c3.9 .2 7.8 .4 11.7 .4zm2.3-48l19.7-67c33.5-9.6 58-40.4 58-76.9c0-44.2-35.8-80-80-80c-36.5 0-67.4 24.5-76.9 58L112 253.7C113.2 175.2 177.2 112 256 112c79.5 0 144 64.5 144 144c0 78.8-63.2 142.8-141.7 144zM39 308.5l204.8-60.2c12.1-3.6 23.4 7.7 19.9 19.9L203.5 473c-4.1 13.9-23.2 15.6-29.7 2.6l-28.7-57.3c-.7-1.3-1.5-2.6-2.5-3.7l-88 88c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l88-88c-1.1-1-2.3-1.9-3.7-2.5L36.4 338.2c-13-6.5-11.3-25.6 2.6-29.7z";
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
    exports.faBullseyePointer = exports.definition;
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

// src/_entries/faBullseyePointer.js
var import_faBullseyePointer = __toESM(require_faBullseyePointer());
var export_faBullseyePointer = import_faBullseyePointer.definition;
export {
  export_faBullseyePointer as faBullseyePointer
};
