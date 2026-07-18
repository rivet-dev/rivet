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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUsersLine.js
var require_faUsersLine = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUsersLine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "users-line";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e592";
    var svgPathData = "M224 128a64 64 0 1 1 128 0 64 64 0 1 1 -128 0zM176 336c0-61.9 50.1-112 112-112s112 50.1 112 112l0 8c0 13.3-10.7 24-24 24l-176 0c-13.3 0-24-10.7-24-24l0-8zM392 144a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zm27.2 100.4c9.1-2.9 18.8-4.4 28.8-4.4 53 0 96 43 96 96l0 10.7c0 11.8-9.6 21.3-21.3 21.3l-78.8 0c2.7-7.5 4.1-15.6 4.1-24l0-8c0-34.1-10.6-65.7-28.8-91.6zm-262.4 0c-18.2 26-28.8 57.5-28.8 91.6l0 8c0 8.4 1.4 16.5 4.1 24l-78.8 0C41.6 368 32 358.4 32 346.7L32 336c0-53 43-96 96-96 10 0 19.7 1.5 28.8 4.4zM72 144a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM0 440c0-13.3 10.7-24 24-24l528 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 464c-13.3 0-24-10.7-24-24z";
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
    exports.faUsersLine = exports.definition;
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

// src/_entries/faUsersLine.js
var import_faUsersLine = __toESM(require_faUsersLine());
var export_faUsersLine = import_faUsersLine.definition;
export {
  export_faUsersLine as faUsersLine
};
