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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDeer.js
var require_faDeer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDeer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "deer";
    var width = 512;
    var height = 512;
    var aliases = [129420];
    var unicode = "f78e";
    var svgPathData = "M240 0c8.8 0 16 7.2 16 16l0 24c0 13.3 10.7 24 24 24l8 0 8 0c13.3 0 24-10.7 24-24l0-24c0-8.8 7.2-16 16-16s16 7.2 16 16l0 24c0 8.6-1.9 16.7-5.4 24l30.6 0L416 64c13.3 0 24-10.7 24-24l0-24c0-8.8 7.2-16 16-16s16 7.2 16 16l0 24c0 21.7-12.3 40.4-30.3 49.8c.6 .5 1.1 1.1 1.7 1.6l55.3 55.3c8.5 8.5 13.3 20 13.3 32c0 25-20.3 45.3-45.3 45.3L416 224l-5.3 0L384 304l0 64 0 112c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-112-36.3 0c-28.8 0-55.3-9.8-76.3-26.4c-2.1 16.9-9.8 32.6-21.9 44.8L142.9 397c-4.6 4.6-6.2 11.5-4 17.7l19.3 54.7c7.3 20.8-8.1 42.6-30.2 42.6l-33.9 0c-13.6 0-25.7-8.6-30.2-21.4l-15.5-44c-11.4-32.4-7.4-67.8 10-96.3C65 339.4 65 324.9 56 315.9c-3-3-5.7-6.1-8.1-9.5c-1.3 12.9-12.7 22.5-25.7 21.5C8.9 326.9-.9 315.4 .1 302.2L2.4 272c4.2-54.2 49.3-96 103.7-96l7.9 0 14.1 0 64 0c.6 0 1.2 0 1.7 0c.8 0 1.7 0 2.5 0l106.1 0 9.6-32-38 0c-9.9 0-18-8-18-18c0-8.2 5.6-15.4 13.6-17.4L320 96l-24 0-8 0-8 0c-30.9 0-56-25.1-56-56l0-24c0-8.8 7.2-16 16-16zM400 160a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faDeer = exports.definition;
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

// src/_entries/faDeer.js
var import_faDeer = __toESM(require_faDeer());
var export_faDeer = import_faDeer.definition;
export {
  export_faDeer as faDeer
};
