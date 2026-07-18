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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPenSwirl.js
var require_faPenSwirl = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPenSwirl.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-swirl";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e214";
    var svgPathData = "M347.8 17C319.3 6 288.4 0 256 0C114.6 0 0 114.6 0 256S114.6 512 256 512c17.7 0 32-14.3 32-32s-14.3-32-32-32C150 448 64 362 64 256S150 64 256 64c14.1 0 27.8 1.5 41 4.4L347.8 17zM310.6 349.4c-5.4 5.3-11.4 9.8-17.8 13.6c-1.7 2.7-3 5.7-3.8 9c-4.4 17.1 5.9 34.6 23 39c12.8 3.3 26.2 5 40 5c88.4 0 160-71.6 160-160c0-32.2-5.9-63-16.8-91.3l-51.4 51.4c2.7 12.9 4.2 26.2 4.2 39.9c0 53-43 96-96 96c-8.3 0-16.4-1.1-24-3c-5.9-1.5-11.9-1.3-17.4 .4zM473.9 16c-18.8-18.8-49.2-18.7-67.9 0L366.4 55.7l89.3 89.3 39.8-38.9c19.1-18.7 19.2-49.4 .4-68.2L473.9 16zM200.5 221.8c-7 7-12.3 15.6-15.4 25l-23.8 71.3c-2.9 8.6-.6 18.1 5.8 24.6s15.9 8.7 24.6 5.8l71.5-23.9c9.2-3.1 17.6-8.2 24.5-15L432.8 167.5 343.7 78.4 200.5 221.8z";
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
    exports.faPenSwirl = exports.definition;
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

// src/_entries/faPenSwirl.js
var import_faPenSwirl = __toESM(require_faPenSwirl());
var export_faPenSwirl = import_faPenSwirl.definition;
export {
  export_faPenSwirl as faPenSwirl
};
