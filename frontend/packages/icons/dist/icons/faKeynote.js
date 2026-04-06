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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKeynote.js
var require_faKeynote = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKeynote.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "keynote";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f66c";
    var svgPathData = "M184 72l14.4 0c8.3 14.3 23.8 24 41.6 24l64 0c26.5 0 48-21.5 48-48s-21.5-48-48-48L240 0c-17.8 0-33.3 9.7-41.6 24L184 24c-48.6 0-88 39.4-88 88l0 48-16 0c-12.1 0-23.2 6.8-28.6 17.7l-48 96C1.2 278.1 0 283 0 288l0 32c0 17.7 14.3 32 32 32l448 0c17.7 0 32-14.3 32-32l0-32c0-5-1.2-9.9-3.4-14.3l-48-96C455.2 166.8 444.1 160 432 160l-288 0 0-48c0-22.1 17.9-40 40-40zM160 448c-17.7 0-32 14.3-32 32s14.3 32 32 32l96 0 96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-64 0 0-64-64 0 0 64-64 0z";
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
    exports.faKeynote = exports.definition;
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

// src/_entries/faKeynote.js
var import_faKeynote = __toESM(require_faKeynote());
var export_faKeynote = import_faKeynote.definition;
export {
  export_faKeynote as faKeynote
};
