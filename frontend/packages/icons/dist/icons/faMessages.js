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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMessages.js
var require_faMessages = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMessages.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "messages";
    var width = 640;
    var height = 512;
    var aliases = ["comments-alt"];
    var unicode = "f4b6";
    var svgPathData = "M64 0C28.7 0 0 28.7 0 64L0 256c0 35.3 28.7 64 64 64l32 0 0 48c0 6.1 3.4 11.6 8.8 14.3s11.9 2.1 16.8-1.5L202.7 320 352 320c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64L64 0zM352 352l-96 0 0 32c0 35.3 28.7 64 64 64l117.3 0 81.1 60.8c4.8 3.6 11.3 4.2 16.8 1.5s8.8-8.2 8.8-14.3l0-48 32 0c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64l-128 0 0 128c0 53-43 96-96 96z";
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
    exports.faMessages = exports.definition;
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

// src/_entries/faMessages.js
var import_faMessages = __toESM(require_faMessages());
var export_faMessages = import_faMessages.definition;
export {
  export_faMessages as faMessages
};
