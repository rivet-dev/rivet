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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSnowmanHead.js
var require_faSnowmanHead = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSnowmanHead.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "snowman-head";
    var width = 448;
    var height = 512;
    var aliases = ["frosty-head"];
    var unicode = "f79b";
    var svgPathData = "M64 64C64 28.7 92.7 0 128 0L320 0c35.3 0 64 28.7 64 64l0 96-160 0L64 160l0-96zM330.2 192l85.8 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-25.7 0c16.3 28.2 25.7 61 25.7 96c0 59.8-27.4 113.3-70.3 148.5c-9.7 8-22.3 11.5-34.9 11.5l-173.7 0c-12.6 0-25.1-3.5-34.9-11.5C59.4 465.3 32 411.8 32 352c0-35 9.3-67.8 25.7-96L32 256c-17.7 0-32-14.3-32-32s14.3-32 32-32l85.8 0 212.3 0zM152 336a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm168-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM219.3 445.2c.9 1.7 2.7 2.8 4.7 2.8s3.8-1.1 4.7-2.8l23.8-46.3c2.3-4.5 3.5-9.6 3.5-14.7c0-17.7-14.3-32.2-32-32.2s-32 14.5-32 32.2c0 5.1 1.2 10.2 3.5 14.7l23.8 46.3z";
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
    exports.faSnowmanHead = exports.definition;
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

// src/_entries/faSnowmanHead.js
var import_faSnowmanHead = __toESM(require_faSnowmanHead());
var export_faSnowmanHead = import_faSnowmanHead.definition;
export {
  export_faSnowmanHead as faSnowmanHead
};
