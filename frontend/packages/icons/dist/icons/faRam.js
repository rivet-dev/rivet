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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRam.js
var require_faRam = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRam.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ram";
    var width = 640;
    var height = 512;
    var aliases = [128015];
    var unicode = "f70a";
    var svgPathData = "M336 64l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8c0-8.8 7.2-16 16-16s16 7.2 16 16l0 24.4c-6.8 6.1-12.8 13.2-17.6 21l-47.8 20.5c-12.2 5.2-17.8 19.3-12.6 31.5s19.3 17.8 31.5 12.6l14.5-6.2 0 72.2c0 44.2 35.8 80 80 80l32 0c44.2 0 80-35.8 80-80l0-72.2 14.5 6.2c12.2 5.2 26.3-.4 31.5-12.6s-.4-26.3-12.6-31.5l-47.8-20.5c-4.9-7.8-10.8-14.9-17.6-21L544 64c0-8.8 7.2-16 16-16s16 7.2 16 16l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-8c0-35.3-28.7-64-64-64s-64 28.7-64 64l0 1.3c-5.2-.9-10.5-1.3-16-1.3s-10.8 .5-16 1.3l0-1.3c0-35.3-28.7-64-64-64s-64 28.7-64 64zM308.5 174.1c-8.9-20.8-4.3-44 10-59.7C312.3 85.6 286.7 64 256 64c-19.2 0-36.5 8.5-48.2 21.9C196.9 77.2 183 72 168 72c-30.1 0-55.3 20.8-62.2 48.8c-3.2-.5-6.5-.8-9.8-.8c-35.3 0-64 28.7-64 64c0 5.3 .7 10.5 1.9 15.5C13.7 210.3 0 231.5 0 256s13.7 45.7 33.9 56.5c-1.2 5-1.9 10.2-1.9 15.5c0 35.3 28.7 64 64 64c.5 0 1 0 1.5 0l17.6 93.9c2.8 15.1 16.1 26.1 31.5 26.1l29.4 0c17.7 0 32-14.3 32-32l0-53.7c11.7 13.3 28.9 21.7 48 21.7s36.3-8.4 48-21.7l0 53.7c0 17.7 14.3 32 32 32l29.4 0c15.4 0 28.6-11 31.4-26.1L414.5 392c.5 0 1 0 1.5 0c26.9 0 49.9-16.5 59.3-40L464 352c-61.9 0-112-50.1-112-112l0-32.6c-18.7-2.7-35.5-14.8-43.5-33.4zM464 160a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm48 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0z";
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
    exports.faRam = exports.definition;
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

// src/_entries/faRam.js
var import_faRam = __toESM(require_faRam());
var export_faRam = import_faRam.definition;
export {
  export_faRam as faRam
};
