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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSealQuestion.js
var require_faSealQuestion = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSealQuestion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "seal-question";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e243";
    var svgPathData = "M222.1 17.9c18.7-18.7 49.1-18.7 67.9 0L336 64l64 0c26.5 0 48 21.5 48 48l0 64 45.6 45.6c18.7 18.7 18.7 49.1 0 67.9L448 335.1l0 64.9c0 26.5-21.5 48-48 48l-64.9 0-45.1 45.1c-18.7 18.7-49.1 18.7-67.9 0L176.9 448 112 448c-26.5 0-48-21.5-48-48l0-64.9L18.4 289.5c-18.7-18.7-18.7-49.1 0-67.9L64 176l0-64c0-26.5 21.5-48 48-48l64 0 46.1-46.1zM169.8 165.3l-.4 1.2c-4.4 12.5 2.1 26.2 14.6 30.6s26.2-2.1 30.6-14.6l.4-1.2c1.1-3.2 4.2-5.3 7.5-5.3l58.3 0c8.4 0 15.1 6.8 15.1 15.1c0 5.4-2.9 10.4-7.6 13.1l-44.3 25.4c-7.5 4.3-12.1 12.2-12.1 20.8l0 13.5c0 13.3 10.7 24 24 24c13.1 0 23.8-10.5 24-23.6l32.3-18.5c19.6-11.3 31.7-32.2 31.7-54.8c0-34.9-28.3-63.1-63.1-63.1l-58.3 0c-23.7 0-44.8 14.9-52.8 37.3zM288 352a32 32 0 1 0 -64 0 32 32 0 1 0 64 0z";
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
    exports.faSealQuestion = exports.definition;
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

// src/_entries/faSealQuestion.js
var import_faSealQuestion = __toESM(require_faSealQuestion());
var export_faSealQuestion = import_faSealQuestion.definition;
export {
  export_faSealQuestion as faSealQuestion
};
