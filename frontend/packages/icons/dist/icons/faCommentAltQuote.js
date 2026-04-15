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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageQuote.js
var require_faMessageQuote = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageQuote.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "message-quote";
    var width = 512;
    var height = 512;
    var aliases = ["comment-alt-quote"];
    var unicode = "e1e4";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L448 0c35.3 0 64 28.7 64 64l0 288c0 35.3-28.7 64-64 64l-138.7 0L185.6 508.8c-4.8 3.6-11.3 4.2-16.8 1.5s-8.8-8.2-8.8-14.3l0-80-96 0c-35.3 0-64-28.7-64-64L0 64zm160 48c-17.7 0-32 14.3-32 32l0 48c0 17.7 14.3 32 32 32l32 0 0 7.3c0 11.7-8.5 21.7-20.1 23.7l-7.9 1.3c-13.1 2.2-21.9 14.5-19.7 27.6s14.5 21.9 27.6 19.7l7.9-1.3c34.7-5.8 60.2-35.8 60.2-71l0-39.3 0-24 0-24c0-17.7-14.3-32-32-32l-48 0zm224 80l0-24 0-24c0-17.7-14.3-32-32-32l-48 0c-17.7 0-32 14.3-32 32l0 48c0 17.7 14.3 32 32 32l32 0 0 7.3c0 11.7-8.5 21.7-20.1 23.7l-7.9 1.3c-13.1 2.2-21.9 14.5-19.7 27.6s14.5 21.9 27.6 19.7l7.9-1.3c34.7-5.8 60.2-35.8 60.2-71l0-39.3z";
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
    exports.faMessageQuote = exports.definition;
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

// src/_entries/faCommentAltQuote.js
var import_faMessageQuote = __toESM(require_faMessageQuote());
var export_faCommentAltQuote = import_faMessageQuote.definition;
export {
  export_faCommentAltQuote as faCommentAltQuote
};
