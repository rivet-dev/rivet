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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRepeat1.js
var require_faRepeat1 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRepeat1.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "repeat-1";
    var width = 512;
    var height = 512;
    var aliases = [128258];
    var unicode = "f365";
    var svgPathData = "M32 256c-17.7 0-32-14.3-32-32C0 135.6 71.6 64 160 64l160 0 0-32c0-12.9 7.8-24.6 19.8-29.6s25.7-2.2 34.9 6.9l64 64c12.5 12.5 12.5 32.8 0 45.3l-64 64c-9.2 9.2-22.9 11.9-34.9 6.9s-19.8-16.6-19.8-29.6l0-32-160 0c-53 0-96 43-96 96c0 17.7-14.3 32-32 32zm448 0c17.7 0 32 14.3 32 32c0 88.4-71.6 160-160 160l-160 0 0 32c0 12.9-7.8 24.6-19.8 29.6s-25.7 2.2-34.9-6.9l-64-64c-12.5-12.5-12.5-32.8 0-45.3l64-64c9.2-9.2 22.9-11.9 34.9-6.9s19.8 16.6 19.8 29.6l0 32 160 0c53 0 96-43 96-96c0-17.7 14.3-32 32-32zM288 216l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-48c-10 0-19.4-6.4-22.8-16.4c-4.2-12.6 2.6-26.2 15.2-30.4l24-8c7.3-2.4 15.4-1.2 21.6 3.3s10 11.8 10 19.5z";
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
    exports.faRepeat1 = exports.definition;
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

// src/_entries/faRepeat1.js
var import_faRepeat1 = __toESM(require_faRepeat1());
var export_faRepeat1 = import_faRepeat1.definition;
export {
  export_faRepeat1 as faRepeat1
};
