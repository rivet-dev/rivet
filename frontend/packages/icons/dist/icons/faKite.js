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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKite.js
var require_faKite = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKite.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kite";
    var width = 640;
    var height = 512;
    var aliases = [129665];
    var unicode = "f6f4";
    var svgPathData = "M344 0c-14.9 0-27.8 10.3-31.2 24.8l-80 344c-.4 1.7-.7 3.5-.8 5.2l-85.4 85.4c-2.9 2.9-6.9 4.6-11 4.6c-8.6 0-15.6-7-15.6-15.6l0-88.6-24-12-24 12 0 88.6c0 35.1 28.5 63.6 63.6 63.6c16.9 0 33-6.7 45-18.6L266 407.9c1.8-.1 3.5-.4 5.2-.8l344-80c14.5-3.4 24.8-16.3 24.8-31.2l0-264c0-17.7-14.3-32-32-32L344 0zM24 128c-13.3 0-24 10.7-24 24s10.7 24 24 24l8 0c22.1 0 40 17.9 40 40l0 68L18.7 257.4c-1.8-.9-3.8-1.4-5.8-1.4C5.8 256 0 261.8 0 268.9l0 70.1C0 346.2 5.8 352 12.9 352c2 0 4-.5 5.8-1.4L72 324s0 0 0 0l9.7-4.8c4.5-2.3 9.4-3.4 14.3-3.4l.5 0c3.5 .1 7 .7 10.4 1.9c1.2 .4 2.3 .9 3.4 1.5c0 0 0 0 0 0L120 324l53.3 26.6c1.8 .9 3.8 1.4 5.8 1.4c7.1 0 12.9-5.8 12.9-12.9l0-70.1c0-7.1-5.8-12.9-12.9-12.9c-2 0-4 .5-5.8 1.4L120 284l0-68c0-48.6-39.4-88-88-88l-8 0zM427.3 84.7L480 137.4l52.7-52.7c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6L502.6 160l52.7 52.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L480 182.6 363.3 299.3c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6L457.4 160l-52.7-52.7c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0z";
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
    exports.faKite = exports.definition;
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

// src/_entries/faKite.js
var import_faKite = __toESM(require_faKite());
var export_faKite = import_faKite.definition;
export {
  export_faKite as faKite
};
