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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraWebSlash.js
var require_faCameraWebSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraWebSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-web-slash";
    var width = 640;
    var height = 512;
    var aliases = ["webcam-slash"];
    var unicode = "f833";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-135.2-106C525.9 324.9 544 276.6 544 224C544 100.3 443.7 0 320 0C248 0 183.9 34 143 86.7L38.8 5.1zM193.3 126.2C222.6 88.4 268.5 64 320 64c88.4 0 160 71.6 160 160c0 37.7-13 72.3-34.8 99.6L420 303.9c17.5-21.9 28-49.7 28-79.9c0-70.7-57.3-128-128-128c-41.3 0-78.1 19.6-101.5 50l-25.2-19.7zm88.2 69.1l-25.2-19.7C270.9 156.4 294 144 320 144c8.8 0 16 7.2 16 16s-7.2 16-16 16c-15.8 0-29.8 7.6-38.5 19.3zM507.7 496L359.4 379.1c-12.6 3.2-25.8 4.9-39.4 4.9c-88.4 0-160-71.6-160-160c0-.6 0-1.3 0-1.9l-58.8-46.3C97.8 191.3 96 207.4 96 224c0 79.5 41.4 149.3 103.8 189.1l-58.2 40.7c-11.4 8-16.4 22.5-12.2 35.8S146 512 160 512l320 0c11.6 0 22.1-6.2 27.7-16zM324.8 351.9L194.5 249.2C206.2 307.8 257.9 352 320 352c1.6 0 3.2 0 4.8-.1z";
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
    exports.faCameraWebSlash = exports.definition;
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

// src/_entries/faWebcamSlash.js
var import_faCameraWebSlash = __toESM(require_faCameraWebSlash());
var export_faWebcamSlash = import_faCameraWebSlash.definition;
export {
  export_faWebcamSlash as faWebcamSlash
};
