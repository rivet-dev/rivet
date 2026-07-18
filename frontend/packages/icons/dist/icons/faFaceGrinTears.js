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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinTears.js
var require_faFaceGrinTears = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceGrinTears.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-grin-tears";
    var width = 640;
    var height = 512;
    var aliases = [128514, "grin-tears"];
    var unicode = "f588";
    var svgPathData = "M554.8 203.4l16.3 2.3C547.7 88.4 444.2 0 320 0S92.3 88.4 68.9 205.7l16.3-2.3c37-5.3 68.6 26.4 63.4 63.4l-6.7 46.7c-4.1 29-21.6 53.2-45.8 66.8 43.7 78.5 127.6 131.7 223.8 131.7s180.1-53.1 223.8-131.7c-24.1-13.5-41.6-37.8-45.8-66.8l-6.7-46.7c-5.3-37 26.4-68.6 63.4-63.4zM470.3 326.9C447.9 388.2 389.1 432 320 432S192.1 388.2 169.7 326.9c-4.3-11.8 7.7-21.8 19.6-18.1 39.2 12.2 83.7 19.1 130.7 19.1s91.5-6.9 130.7-19.1c11.9-3.7 23.9 6.3 19.6 18.1zM212 208l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-37.6 30.4-68 68-68s68 30.4 68 68l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-15.5-12.5-28-28-28s-28 12.5-28 28zm188-28c-15.5 0-28 12.5-28 28l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-37.6 30.4-68 68-68s68 30.4 68 68l0 8c0 11-9 20-20 20s-20-9-20-20l0-8c0-15.5-12.5-28-28-28zM640 300.6c0-25.6-18.8-47.3-44.1-50.9L549.1 243c-10.6-1.5-19.6 7.5-18.1 18.1l6.7 46.7c3.6 25.3 25.3 44.1 50.9 44.1 28.4 0 51.4-23 51.4-51.4zm-640 0C0 329 23 352 51.4 352 77 352 98.7 333.2 102.3 307.9l6.7-46.7c1.5-10.6-7.5-19.6-18.1-18.1l-46.7 6.7C18.8 253.3 0 275 0 300.6z";
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
    exports.faFaceGrinTears = exports.definition;
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

// src/_entries/faFaceGrinTears.js
var import_faFaceGrinTears = __toESM(require_faFaceGrinTears());
var export_faFaceGrinTears = import_faFaceGrinTears.definition;
export {
  export_faFaceGrinTears as faFaceGrinTears
};
