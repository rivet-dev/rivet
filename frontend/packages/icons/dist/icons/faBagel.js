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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBagel.js
var require_faBagel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBagel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bagel";
    var width = 640;
    var height = 512;
    var aliases = [129391];
    var unicode = "e3d7";
    var svgPathData = "M640 208c0 114.9-93.1 208-208 208c-3.9 0-7.7-.1-11.5-.3c6.5-12.3 12-25.3 16.3-38.8c4.7-.7 9.3-2.5 13.3-5.3L471 357.3c5.1-3.5 11-5.4 17.2-5.6l25.3-.7c13.2-.4 24.8-8.8 29.3-21.3l8.5-23.9c2.1-5.8 5.8-10.9 10.6-14.6L582 275.8c10.5-8.1 14.9-21.7 11.2-34.4L586 217c-1.7-5.9-1.7-12.2 0-18.1l7.2-24.3c3.7-12.7-.7-26.3-11.2-34.4l-20.1-15.5c-4.9-3.8-8.6-8.8-10.6-14.6l-8.5-23.9c-4.4-12.5-16.1-20.9-29.3-21.3l-25.3-.7c-6.2-.2-12.1-2.1-17.2-5.6L450.1 44.4c-10.9-7.5-25.3-7.5-36.2 0L393 58.7c-5.1 3.5-11 5.4-17.2 5.6L350.5 65c-13.2 .4-24.8 8.8-29.3 21.3l-1.8 5.1c-14.4-7.6-29.7-13.7-45.7-18.3C311.8 28.4 368.6 0 432 0C546.9 0 640 93.1 640 208zM396.1 155c10.2-6.9 22.6-11 35.9-11c35.3 0 64 28.7 64 64c0 30.6-21.5 56.3-50.3 62.5c-6-43.3-23.6-82.9-49.5-115.6zM0 304a208 208 0 1 1 416 0A208 208 0 1 1 0 304zm256 0a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM112 416a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm144 16a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48-16a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm16-176a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM144 224a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM96 272a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faBagel = exports.definition;
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

// src/_entries/faBagel.js
var import_faBagel = __toESM(require_faBagel());
var export_faBagel = import_faBagel.definition;
export {
  export_faBagel as faBagel
};
