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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceIcicles.js
var require_faFaceIcicles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceIcicles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-icicles";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e37c";
    var svgPathData = "M512 256c0 8.3-.4 16.6-1.2 24.7c0 .5-.1 1.1-.1 1.6l-14.8 200C494.7 499.1 480.8 512 464 512s-30.7-12.9-31.9-29.6l-2.8-37.9c-5 4.6-10.2 9-15.6 13.2l-11.5 32.9C397.7 503.4 385.6 512 372 512c-11.5 0-21.9-6.1-27.5-15.7C316.9 506.5 287.1 512 256 512c-69.4 0-132.3-27.6-178.4-72.4C71.9 444.9 64.3 448 56 448c-15.6 0-29-11.3-31.6-26.7L6.5 313.4C2.2 294.9 0 275.7 0 256C0 114.6 114.6 0 256 0S512 114.6 512 256zM56 415.8l0 .2 0-.2 20-120.2C78.1 283.3 68.6 272 56 272s-22.1 11.3-20.1 23.7L56 415.8zM344 400l28 80 28-80 0-40-56 0 0 40zm55.3-56c-3.8-22.7-23.6-40-47.3-40l-8 0 0 40 55.3 0zM328 344l0-40-64 0 0 40 64 0zm0 56l0-40-64 0 0 40 32 48 32-48zm-80-96l-64 0 0 40 64 0 0-40zm0 56l-64 0 0 40 64 0 0-40zm-80-16l0-40-8 0c-23.8 0-43.5 17.3-47.3 40l55.3 0zm0 56l0-40-55.3 0c3.8 22.7 23.6 40 47.3 40l8 0zm40.4-192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM464 480l21.8-232.1C487 235.1 476.9 224 464 224s-23 11.1-21.8 23.9L464 480z";
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
    exports.faFaceIcicles = exports.definition;
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

// src/_entries/faFaceIcicles.js
var import_faFaceIcicles = __toESM(require_faFaceIcicles());
var export_faFaceIcicles = import_faFaceIcicles.definition;
export {
  export_faFaceIcicles as faFaceIcicles
};
