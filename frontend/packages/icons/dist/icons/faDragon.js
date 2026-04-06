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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDragon.js
var require_faDragon = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDragon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dragon";
    var width = 640;
    var height = 512;
    var aliases = [128009];
    var unicode = "f6d5";
    var svgPathData = "M352 124.5l-51.9-13c-6.5-1.6-11.3-7.1-12-13.8s2.8-13.1 8.7-16.1l40.8-20.4-43.3-32.5c-5.5-4.1-7.8-11.3-5.6-17.9S297.1 0 304 0L464 0c30.2 0 58.7 14.2 76.8 38.4l57.6 76.8c6.2 8.3 9.6 18.4 9.6 28.8 0 26.5-21.5 48-48 48l-21.5 0c-17 0-33.3-6.7-45.3-18.7l-13.3-13.3-32 0 0 21.5c0 24.8 12.8 47.9 33.8 61.1l106.6 66.6c32.1 20.1 51.6 55.2 51.6 93.1 0 60.6-49.1 109.8-109.8 109.8L32.3 512c-3.3 0-6.6-.4-9.6-1.4-9.2-2.8-16.7-9.6-20.4-18.6-1.3-3.3-2.2-6.9-2.3-10.7-.2-3.7 .3-7.3 1.3-10.7 2.8-9.2 9.6-16.7 18.6-20.4 3-1.2 6.2-2 9.5-2.2L433.3 412c8.3-.7 14.7-7.7 14.7-16.1 0-4.3-1.7-8.4-4.7-11.4l-44.4-44.4c-30-30-46.9-70.7-46.9-113.1l0-102.5zM512 72.3c0-.1 0-.2 0-.3s0-.2 0-.3l0 .6zm-1.3 7.4L464.3 68.1c-.2 1.3-.3 2.6-.3 3.9 0 13.3 10.7 24 24 24 10.6 0 19.5-6.8 22.7-16.3zM130.9 116.5c16.3-14.5 40.4-16.2 58.5-4.1l130.6 87 0 27.5c0 32.8 8.4 64.8 24 93l-232 0c-6.7 0-12.7-4.2-15-10.4s-.5-13.3 4.6-17.7L171 232.3 18.4 255.8c-7 1.1-13.9-2.6-16.9-9S.1 232.8 5.4 228L130.9 116.5z";
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
    exports.faDragon = exports.definition;
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

// src/_entries/faDragon.js
var import_faDragon = __toESM(require_faDragon());
var export_faDragon = import_faDragon.definition;
export {
  export_faDragon as faDragon
};
