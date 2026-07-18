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

// src/node_modules/@fortawesome/free-brands-svg-icons/faBuffer.js
var require_faBuffer = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faBuffer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "buffer";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f837";
    var svgPathData = "M428.2 380.7L231.7 478.5c-2.3 1-4.8 1.5-7.3 1.5s-5-.5-7.3-1.5L20.5 380.7c-4-2-4-5.3 0-7.3L67.6 350c2.3-1 4.8-1.5 7.3-1.5s5 .5 7.3 1.5l134.8 67c2.3 1 4.8 1.5 7.3 1.5s5-.5 7.3-1.5l134.8-67c2.3-1 4.8-1.5 7.3-1.5s5 .5 7.3 1.5l47.1 23.4c4 2 4 5.2 0 7.2zm0-136.5l-47.1-23.4c-2.3-1-4.8-1.5-7.3-1.5s-5 .5-7.3 1.5L231.7 287.8c-2.3 1-4.8 1.5-7.3 1.5s-5-.5-7.3-1.5L82.3 220.7c-2.3-1-4.8-1.5-7.3-1.5s-5 .5-7.3 1.5L20.5 244.1c-4 2-4 5.3 0 7.3l196.5 97.8c2.3 1 4.8 1.5 7.3 1.5s5-.5 7.3-1.5l196.5-97.8c4-2 4-5.3 0-7.3zM20.5 130.4L217 220.7c4.7 1.9 10 1.9 14.7 0l196.5-90.3c4-1.9 4-4.9 0-6.7L231.7 33.4c-4.7-1.9-10-1.9-14.7 0L20.5 123.7c-4 1.8-4 4.9 0 6.7z";
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
    exports.faBuffer = exports.definition;
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

// src/_entries/faBuffer.js
var import_faBuffer = __toESM(require_faBuffer());
var export_faBuffer = import_faBuffer.definition;
export {
  export_faBuffer as faBuffer
};
